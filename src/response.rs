use hyper::{header, Response, StatusCode};

// CANNED RESPONSES

pub fn bad_request(msg: &'static str) -> Response {
    Response::new()
        .with_status(StatusCode::BadRequest)
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(msg.len() as u64))
        .with_body(msg)
}

#[allow(dead_code)]
pub fn method_not_allowed() -> Response {
    const TEXT: &str = "Only GET, HEAD, OPTIONS allowed";
    Response::new()
        .with_status(StatusCode::MethodNotAllowed)
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(TEXT.len() as u64))
        .with_body(TEXT)
}

pub fn not_found() -> Response {
    const TEXT: &str = "Not found";
    Response::new()
        .with_status(StatusCode::NotFound)
        .with_header(header::ContentLength(TEXT.len() as u64))
        .with_header(header::ContentType::plaintext())
        .with_body(TEXT)
}

pub fn internal_server_error() -> Response {
    const TEXT: &str = "Internal server error";
    Response::new()
        .with_status(StatusCode::InternalServerError)
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(TEXT.len() as u64))
        .with_body(TEXT)
}

pub fn precondition_failed() -> Response {
    Response::new()
        .with_status(StatusCode::PreconditionFailed)
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(0))
}

pub fn not_modified(etag: header::EntityTag) -> Response {
    Response::new()
        .with_status(StatusCode::NotModified)
        .with_header(header::ETag(etag)) // Required in 304 response
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(0))
}

pub fn invalid_range(entity_len: u64) -> Response {
    const TEXT: &str = "Invalid range";
    Response::new()
        .with_status(StatusCode::RangeNotSatisfiable)
        .with_header(header::ContentRange(header::ContentRangeSpec::Bytes {
            range: None,
            instance_length: Some(entity_len),
        }))
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(TEXT.len() as u64))
        .with_body(TEXT)
}
