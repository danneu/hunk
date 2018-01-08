#![feature(ip_constructors)]

extern crate flate2;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate unicase;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

// 3rd party

use futures::{stream, Future, Sink, Stream};
use futures_cpupool::CpuPool;

use hyper::{Chunk, Method, StatusCode};
use hyper::server::{Request, Response};
use hyper::header;
use hyper::mime as hypermime;

use flate2::write::GzEncoder;

use std::fs::File;
use std::sync::Arc;
use std::io::{self, Write};
use std::ops::Range;
use std::time;
use std::path::{self, Path, PathBuf};
use std::os::unix::fs::FileExt;
use std::collections::HashSet;

// 1st party

mod range;
mod negotiation;
mod mime;
mod base36;
pub mod config;
pub mod options;

use range::RequestedRange;

const CHUNK_SIZE: u64 = 65_536;

pub struct Context {
    pub root: PathBuf,
    pub pool: CpuPool,
    pub opts: options::Options,
}

pub struct HttpService(&'static Context);

impl HttpService {
    pub fn new(ctx: &'static Context) -> HttpService {
        HttpService(ctx)
    }
}

type ChunkStream = Box<Stream<Item = Chunk, Error = hyper::Error> + Send>;

trait ChunkStreamable {
    fn get_range(&self, range: Range<u64>) -> ChunkStream;
}

struct ResourceInner {
    len: u64,
    mtime: time::SystemTime,
    content_type: ::hypermime::Mime,
    file: File,
    pool: CpuPool,
}

#[derive(Clone)]
struct Resource {
    inner: Arc<ResourceInner>,
}

impl Resource {
    fn new(file: File, pool: CpuPool, content_type: hypermime::Mime) -> Result<Self, io::Error> {
        let m = file.metadata()?;
        Ok(Resource {
            inner: Arc::new(ResourceInner {
                len: m.len(),
                mtime: m.modified()?,
                file,
                pool,
                content_type,
            }),
        })
    }

    fn len(&self) -> u64 {
        self.inner.len
    }

    fn content_type(&self) -> &hypermime::Mime {
        &self.inner.content_type
    }

    fn last_modified(&self) -> header::HttpDate {
        header::HttpDate::from(self.inner.mtime)
    }

    fn etag(&self, strong: bool) -> header::EntityTag {
        let dur = self.inner
            .mtime
            .duration_since(time::UNIX_EPOCH)
            .unwrap_or_else(|_| time::Duration::new(0, 0));

        let tag = format!(
            "{}${}",
            base36::encode(self.len()),
            base36::encode(dur.as_secs()), // TODO: would rather use millis
        );

        if strong {
            header::EntityTag::strong(tag)
        } else {
            header::EntityTag::weak(tag)
        }
    }
}

impl ChunkStreamable for Resource {
    fn get_range(&self, range: Range<u64>) -> ChunkStream {
        let stream =
            futures::stream::unfold((range, Arc::clone(&self.inner)), move |(left, inner)| {
                if left.start == left.end {
                    return None;
                }
                let chunk_size = std::cmp::min(CHUNK_SIZE, left.end - left.start) as usize;
                let mut chunk = Vec::with_capacity(chunk_size);
                unsafe { chunk.set_len(chunk_size) };
                let bytes_read = match inner.file.read_at(&mut chunk, left.start) {
                    Err(e) => return Some(Err(hyper::Error::from(e))),
                    Ok(n) => n,
                };
                chunk.truncate(bytes_read);
                Some(Ok((
                    Chunk::from(chunk),
                    (left.start + bytes_read as u64..left.end, inner),
                )))
            });

        let stream: ChunkStream = {
            let (tx, rx) = ::futures::sync::mpsc::channel(0);
            self.inner.pool.spawn(tx.send_all(stream.then(Ok))).forget();
            Box::new(
                rx.map_err(|()| unreachable!())
                    .and_then(::futures::future::result),
            )
        };

        stream
    }
}

// Gzip each chunk with the given compression level.
fn gzip(body: ChunkStream, level: ::flate2::Compression) -> ChunkStream {
    Box::new(body.and_then(move |chunk| {
        let mut encoder = GzEncoder::new(Vec::new(), level);
        encoder
            .write(chunk.as_ref())
            .and_then(|_| encoder.finish())
            .map(|vec| vec.into())
            .map_err(|e| e.into())
    }))
}

lazy_static! {
    static ref ALLOWED_METHODS: HashSet<Method> = {
        let mut set = HashSet::new();
        set.insert(Method::Get);
        set.insert(Method::Head);
        set.insert(Method::Options);
        set
    };
}

fn is_not_modified(resource: &Resource, req: &Request, resource_etag: &header::EntityTag) -> bool {
    if !negotiation::none_match(req.headers().get::<header::IfNoneMatch>(), resource_etag) {
        true
    } else if let Some(&header::IfModifiedSince(since)) = req.headers().get() {
        resource.last_modified() <= since
    } else {
        false
    }
}

fn is_precondition_failed(
    resource: &Resource,
    req: &Request,
    resource_etag: &header::EntityTag,
) -> bool {
    if !negotiation::any_match(req.headers().get::<header::IfMatch>(), resource_etag) {
        true
    } else if let Some(&header::IfUnmodifiedSince(since)) = req.headers().get() {
        resource.last_modified() > since
    } else {
        false
    }
}

fn handler(ctx: &'static Context, req: &Request) -> Response<ChunkStream> {
    let resource_path = match get_resource_path(&ctx.root, req.uri().path()) {
        None => return not_found(),
        Some(path) => path,
    };

    let file = match File::open(&resource_path) {
        Err(_) => return not_found(),
        Ok(file) => file,
    };

    let resource = match Resource::new(
        file,
        ctx.pool.clone(),
        mime::guess_mime_by_path(resource_path.as_path()),
    ) {
        Err(_) => return not_found(),
        Ok(resource) => resource,
    };

    if !ALLOWED_METHODS.contains(req.method()) {
        return method_not_allowed();
    }

    // HANDLE CACHING HEADERS

    let should_gzip: bool = ctx.opts.gzip.as_ref().map(|opts| {
        resource.len() >= opts.threshold && mime::is_compressible_path(&resource_path) &&
            negotiation::negotiate_encoding(req.headers().get::<header::AcceptEncoding>()) == Some(header::Encoding::Gzip)
    }).unwrap_or(false);

    let resource_etag = resource.etag(!should_gzip);

    if is_not_modified(&resource, req, &resource_etag) {
        return not_modified(resource_etag);
    }

    if is_precondition_failed(&resource, req, &resource_etag) {
        return precondition_failed();
    }

    // PARSE RANGE HEADER
    // - Comes after evaluating precondition headers.
    //   <https://tools.ietf.org/html/rfc7233#section-3.1>

    let range = range::parse_range_header(
        req.headers().has::<header::Range>(),
        req.headers().get::<header::Range>(),
        resource.len(),
    );

    if let RequestedRange::NotSatisfiable = range {
        return invalid_range(resource.len());
    };

    let mut res = Response::new();

    res.headers_mut().set(header::ETag(resource_etag));
    res.headers_mut()
        .set(header::AcceptRanges(vec![header::RangeUnit::Bytes]));
    res.headers_mut()
        .set(header::LastModified(resource.last_modified()));
    res.headers_mut()
        .set(header::ContentType(resource.content_type().to_owned()));

    // Accept-Encoding doesn't affect the response unless gzip is turned on
    if ctx.opts.gzip.is_some() {
        res.headers_mut()
            .set(header::Vary::Items(vec![
                unicase::Ascii::new("Accept-Encoding".to_owned())
            ]));
    }

    // Only set max-age if it's configured at all.
    if let Some(max_age) = ctx.opts.cache.as_ref().map(|opts| opts.max_age) {
        res.headers_mut().set(header::CacheControl(vec![
            header::CacheDirective::Public,
            header::CacheDirective::MaxAge(max_age),
        ]));
    }

    let body = {
        let range = match range {
            RequestedRange::Satisfiable(mut range) => {
                res.set_status(StatusCode::PartialContent);
                res.headers_mut()
                    .set(header::ContentRange(header::ContentRangeSpec::Bytes {
                        range: Some((range.start, range.end)),
                        instance_length: Some(resource.len()),
                    }));

                // NOTE: Range header is end-inclusive but std::ops::Range is end-exclusive.
                range.end += 1;

                range
            }
            _ => 0..resource.len(),
        };

        resource.get_range(range)
    };

    if should_gzip {
        res.headers_mut()
            .set(header::ContentEncoding(vec![header::Encoding::Gzip]));
    }

    // For HEAD requests, we do all the work except sending the body.
    if *req.method() == Method::Head {
        return res;
    }

    if should_gzip {
        res.with_body(gzip(body, ctx.opts.gzip.as_ref().unwrap().level))
    } else {
        res.with_body(body)
    }
}

impl hyper::server::Service for HttpService {
    type Request = Request;
    type Response = Response<ChunkStream>;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let ctx = self.0;
        let work = move || Ok(handler(ctx, &req));
        Box::new(ctx.pool.spawn_fn(work))
    }
}

// Join root with request path to get the asset path candidate.
pub fn get_resource_path(root: &Path, req_path: &str) -> Option<PathBuf> {
    // request path must be absolute
    if !req_path.starts_with('/') {
        return None;
    }

    // request path cannot be a directory {
    if req_path.ends_with('/') {
        return None;
    }

    // Security: request path cannot climb directories
    if Path::new(req_path)
        .components()
        .any(|c| c == path::Component::ParentDir)
    {
        return None;
    };

    let mut final_path = root.to_path_buf();
    final_path.push(&req_path[1..]);

    Some(final_path)
}

// CANNED RESPONSES

fn not_found() -> Response<ChunkStream> {
    let text = b"Not Found";
    let body: ChunkStream = Box::new(stream::once(Ok(text[..].into())));
    Response::new()
        .with_status(StatusCode::NotFound)
        .with_header(header::ContentLength(text.len() as u64))
        .with_body(body)
}

fn precondition_failed() -> Response<ChunkStream> {
    Response::new()
        .with_status(StatusCode::PreconditionFailed)
        .with_header(header::ContentLength(0))
}

fn not_modified(etag: header::EntityTag) -> Response<ChunkStream> {
    Response::new()
        .with_status(StatusCode::NotModified)
        .with_header(header::ETag(etag)) // Required in 304 response
        .with_header(header::ContentLength(0))
}

// TODO: Is OPTIONS part of MethodNotAllowed?
fn method_not_allowed() -> Response<ChunkStream> {
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

fn invalid_range(resource_len: u64) -> Response<ChunkStream> {
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
