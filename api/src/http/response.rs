use bytes::Bytes;
use http::{
    header::{CONTENT_LENGTH, CONTENT_TYPE},
    HeaderValue, StatusCode,
};

pub type Response = http::Response<Option<Bytes>>;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        let mut response = http::Response::new(None);
        *response.status_mut() = self;
        response.headers_mut().insert(CONTENT_LENGTH, 0.into());

        response
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Response {
        let body_len = self.len();
        let mut response = http::Response::new(Some(self));

        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::APPLICATION_OCTET_STREAM.as_ref()),
        );
        response
            .headers_mut()
            .insert(CONTENT_LENGTH, body_len.into());

        response
    }
}

pub struct Json<T>(pub T);
impl<T> IntoResponse for Json<T>
where
    T: serde::Serialize,
{
    fn into_response(self) -> Response {
        let json = serde_json::to_vec(&self.0).unwrap();

        http::Response::builder()
            .header(http::header::CONTENT_TYPE, mime::JSON.as_ref())
            .body(Some(json.into()))
            .expect("failed to create response")
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        IntoResponse::into_response(Bytes::from(self))
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        let mut response = http::Response::new(Some(Bytes::from(self)));

        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
        );

        response
    }
}

impl<B: IntoResponse> IntoResponse for (StatusCode, B) {
    fn into_response(self) -> Response {
        let mut response = self.1.into_response();
        *response.status_mut() = self.0;

        response
    }
}
