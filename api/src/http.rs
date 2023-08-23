pub const LINE_DELIMITER: &[u8] = b"\r\n";
pub const REQUEST_DELIMITER: &[u8] = b"\r\n\r\n";

pub mod codec;
mod response;

pub type Request = http::Request<Option<bytes::Bytes>>;
pub use response::{IntoResponse, Json, Response};
