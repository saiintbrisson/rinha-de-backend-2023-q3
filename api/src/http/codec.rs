use std::{fmt::Write, str::from_utf8};

use bytes::Buf;
use http::{header::CONTENT_LENGTH, request::Builder, Error as HttpError, Method, Uri, Version};
use memchr::memmem;
use once_cell::sync::Lazy;
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    error::{RequestError, ResponseError},
    http::{LINE_DELIMITER, REQUEST_DELIMITER},
};

use super::{Request, Response};

static FINDER: Lazy<memmem::Finder> = Lazy::new(|| memmem::Finder::new(LINE_DELIMITER));

#[derive(Default)]
pub struct ConnectionCodec {
    pub req: Option<(Builder, usize)>,
}

impl Decoder for ConnectionCodec {
    type Item = Request;

    type Error = RequestError;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let (req, len) = match self.req.take() {
            Some(req) => req,
            None => {
                let Some(position) = memmem::find(&src, REQUEST_DELIMITER) else {
                    return Ok(None);
                };

                let req = src.split_to(position);
                let req = request_from_slice(&req)?;
                src.advance(REQUEST_DELIMITER.len());

                let Some(content_length) = req.headers_ref().and_then(|map| map.get(CONTENT_LENGTH)) else {
                    return req.body(None).map(Some).map_err(RequestError::HttpError);
                };

                let content_length = content_length.to_str()?.parse::<usize>()?;

                (req, content_length)
            }
        };

        if src.len() < len {
            src.reserve(len);
            return Ok(None);
        }

        assert_eq!(
            src.len(),
            len,
            "client sent a body larger than reported ({} > {len})",
            src.len()
        );

        req.body(Some(src.split().freeze()))
            .map(Some)
            .map_err(RequestError::HttpError)
    }
}

#[inline]
fn request_from_slice(buf: &[u8]) -> Result<Builder, RequestError> {
    let mut buf = from_utf8(buf)?;
    let mut request_line = split_to_delimiter(&mut buf)?;

    //request line = "METHOD PATH HTTP/VERSION\r\n"
    let method = split_to_byte(&mut request_line, b' ')?;
    let path = split_to_byte(&mut request_line, b' ')?;
    let version = request_line;

    let mut builder = http::Request::builder()
        .method(Method::try_from(method).map_err(HttpError::from)?)
        .uri(Uri::try_from(path).map_err(HttpError::from)?)
        .version(match version {
            "HTTP/0.9" => Version::HTTP_09,
            "HTTP/1.0" => Version::HTTP_10,
            "HTTP/1.1" => Version::HTTP_11,
            "HTTP/2.0" => Version::HTTP_2,
            "HTTP/3.0" => Version::HTTP_3,
            _ => return Err(RequestError::UnsupportedVersion),
        });

    // header = "Name: Value\r\n"
    while let Ok(mut header) = split_to_delimiter(&mut buf) {
        let key = split_to_byte(&mut header, b':')?;
        builder = builder.header(key, header.trim_start());
    }

    Ok(builder)
}

#[inline]
fn split_to_byte<'a>(buf: &mut &'a str, byte: u8) -> Result<&'a str, RequestError> {
    memchr::memchr(byte, buf.as_bytes())
        .map(|e| {
            let part = &buf[..e];
            *buf = &buf[e + 1..];
            part
        })
        .ok_or(RequestError::InvalidFormat)
}

#[inline]
fn split_to_delimiter<'a>(buf: &mut &'a str) -> Result<&'a str, RequestError> {
    if buf.len() == 0 {
        return Err(RequestError::InvalidFormat);
    }

    match FINDER.find(buf.as_bytes()) {
        Some(pos) => {
            let part = &buf[..pos];
            *buf = &buf[pos + LINE_DELIMITER.len()..];
            Ok(part)
        }
        None => {
            let part = &buf[..];
            *buf = &buf[part.len()..];
            Ok(part)
        }
    }
}

impl Encoder<Response> for ConnectionCodec {
    type Error = ResponseError;

    fn encode(&mut self, response: Response, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        write!(dst, "{:?} {:?}\r\n", response.version(), response.status())?;

        for (key, value) in response.headers() {
            let value = value.to_str()?;
            write!(dst, "{}: {}\r\n", key, value)?;
        }

        if response.headers().get(CONTENT_LENGTH).is_none() {
            let len = response
                .body()
                .as_ref()
                .map(|b| b.len())
                .unwrap_or_default();

            write!(dst, "{}: {}\r\n", CONTENT_LENGTH, len)?;
        }

        write!(dst, "\r\n")?;

        if let Some(body) = response.body() {
            dst.extend_from_slice(&body);
        }

        Ok(())
    }
}
