use chunks::ChunkStream;
use hyper::{Response, StatusCode, Method};
use hyper::header;
use futures::stream;

// CANNED RESPONSES

pub fn not_found() -> Response<ChunkStream> {
    let text = b"Not Found";
    let body: ChunkStream = Box::new(stream::once(Ok(text[..].into())));
    Response::new()
        .with_status(StatusCode::NotFound)
        .with_header(header::ContentLength(text.len() as u64))
        .with_body(body)
}

pub fn internal_server_error() -> Response<ChunkStream> {
    let text = b"Internal Server Error";
    let body: ChunkStream = Box::new(stream::once(Ok(text[..].into())));
    Response::new()
        .with_status(StatusCode::InternalServerError)
        .with_header(header::ContentLength(text.len() as u64))
        .with_body(body)
}

pub fn precondition_failed() -> Response<ChunkStream> {
    Response::new()
        .with_status(StatusCode::PreconditionFailed)
        .with_header(header::ContentLength(0))
}

pub fn not_modified(etag: header::EntityTag) -> Response<ChunkStream> {
    Response::new()
        .with_status(StatusCode::NotModified)
        .with_header(header::ETag(etag)) // Required in 304 response
        .with_header(header::ContentLength(0))
}

// TODO: Is OPTIONS part of MethodNotAllowed?
pub fn method_not_allowed() -> Response<ChunkStream> {
    let text = b"This resource only supports GET, HEAD, and OPTIONS.";
    let body: ChunkStream = Box::new(stream::once(Ok(text[..].into())));
    Response::new()
        .with_status(StatusCode::MethodNotAllowed)
        .with_header(header::ContentLength(text.len() as u64))
        .with_header(header::ContentType::plaintext())
        .with_header(header::Allow(vec![
            Method::Get,
            Method::Head,
            Method::Options,
        ]))
        .with_body(body)
}

pub fn invalid_range(resource_len: u64) -> Response<ChunkStream> {
    let text = b"Invalid range";
    let body: ChunkStream = Box::new(stream::once(Ok(text[..].into())));
    Response::new()
        .with_status(StatusCode::RangeNotSatisfiable)
        .with_header(header::ContentRange(header::ContentRangeSpec::Bytes {
            range: None,
            instance_length: Some(resource_len),
        }))
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(text.len() as u64))
        .with_body(body)
}