extern crate flate2;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate mime as hypermime;
extern crate mime_guess;

// 3rd party

use futures::Sink;
use futures::Future;
use futures::Stream;
use futures::stream;

use futures_cpupool::CpuPool;

use hyper::{Method, StatusCode};
use hyper::server::{Request, Response};
use hyper::header::{self, Encoding};

use flate2::Compression;
use flate2::write::GzEncoder;

use std::str::FromStr;
use std::sync::Arc;
use std::io::{self, Write};
use std::ops::Range;
use std::time::{self, Duration, SystemTime};
use std::path::{Component, Path, PathBuf};
use std::os::unix::fs::FileExt;

// 1st party

mod range;
mod codec;
mod negotiation;
mod mime;

use range::RequestedRange;
use codec::base36;

static CHUNK_SIZE: u64 = 65_536;

pub struct Context {
    pub root: PathBuf,
    pub pool: CpuPool,
}

pub struct HttpService(&'static Context);

impl HttpService {
    pub fn new(ctx: &'static Context) -> HttpService {
        HttpService(ctx)
    }
}

trait Entity: 'static + Send {
    type Chunk: 'static + Send + AsRef<[u8]> + From<Vec<u8>> + From<&'static [u8]>;
    type Body: 'static
        + Send
        + Stream<Item = Self::Chunk, Error = hyper::Error>
        + From<Box<Stream<Item = Self::Chunk, Error = hyper::Error> + Send>>;

    /// Returns the length of the entity in bytes.
    fn len(&self) -> u64;

    /// Gets the body bytes indicated by `range`.
    fn get_range(&self, range: Range<u64>, compression: Option<Encoding>) -> Self::Body;

    fn last_modified(&self) -> Option<header::HttpDate>;

    fn etag(&self) -> Option<header::EntityTag>;

    fn content_type(&self) -> &hypermime::Mime;
}

#[derive(Clone)]
pub struct ChunkedFile<B, C> {
    // Arc lets us move across threads.
    inner: Arc<ChunkedFileInner>,
    phantom: ::std::marker::PhantomData<(B, C)>,
}

struct ChunkedFileInner {
    len: u64,
    mtime: SystemTime,
    content_type: ::hypermime::Mime,
    f: std::fs::File,
    pool: CpuPool,
}

impl<B, C> ChunkedFile<B, C> {
    pub fn new(
        file: ::std::fs::File,
        pool: CpuPool,
        content_type: ::hypermime::Mime,
    ) -> Result<Self, io::Error> {
        let m = file.metadata()?;
        Ok(ChunkedFile {
            inner: Arc::new(ChunkedFileInner {
                len: m.len(),
                mtime: m.modified()?,
                content_type,
                f: file,
                pool,
            }),
            phantom: ::std::marker::PhantomData,
        })
    }
}

impl<B, C> Entity for ChunkedFile<B, C>
where
    B: 'static
        + Send
        + Stream<Item = C, Error = hyper::Error>
        + From<Box<Stream<Item = C, Error = hyper::Error> + Send>>,
    C: 'static + Send + AsRef<[u8]> + From<Vec<u8>> + From<&'static [u8]>,
{
    type Chunk = C;
    type Body = B;

    fn len(&self) -> u64 {
        self.inner.len
    }

    fn last_modified(&self) -> Option<header::HttpDate> {
        Some(self.inner.mtime.into())
    }

    fn content_type(&self) -> &hypermime::Mime {
        &self.inner.content_type
    }

    fn etag(&self) -> Option<header::EntityTag> {
        let dur = self.inner
            .mtime
            .duration_since(time::UNIX_EPOCH)
            .unwrap_or(Duration::new(0, 0));
        Some(header::EntityTag::strong(format!(
            "{}${}",
            base36::encode(self.len()),
            base36::encode(dur.as_secs()), // TODO: would rather use millis
        )))
    }

    fn get_range(&self, range: Range<u64>, compression: Option<Encoding>) -> B {
        let stream =
            ::futures::stream::unfold((range, self.inner.clone()), move |(left, inner)| {
                if left.start == left.end {
                    return None;
                }
                let chunk_size = ::std::cmp::min(CHUNK_SIZE, left.end - left.start) as usize;
                let mut chunk = Vec::with_capacity(chunk_size);
                unsafe { chunk.set_len(chunk_size) };
                let bytes_read = match inner.f.read_at(&mut chunk, left.start) {
                    Err(e) => return Some(Err(e.into())),
                    Ok(b) => b,
                };
                chunk.truncate(bytes_read);

                // COMPRESS
                // - NOTE: This of course changes the response size.
                //   Would need to recalc length if I wanted to keep track of content-length.
                //   Haven't looked into it much.
                let chunk = if compression == Some(Encoding::Gzip) {
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder.write(&chunk)
                    .and_then(|_| encoder.finish())
                    // TODO: Handle these potential failures
                    .unwrap()
                } else {
                    chunk
                };

                Some(Ok((
                    chunk.into(),
                    (left.start + bytes_read as u64..left.end, inner),
                )))
            });

        let stream: Box<Stream<Item = C, Error = hyper::Error> + Send> = {
            let (snd, rcv) = ::futures::sync::mpsc::channel(0);
            self.inner
                .pool
                .spawn(snd.send_all(stream.then(|i| Ok(i))))
                .forget();
            Box::new(
                rcv.map_err(|()| unreachable!())
                    .and_then(|r| ::futures::future::result(r)),
            )
        };
        stream.into()
    }
}

fn handler<E: Entity>(path: PathBuf, entity: E, req: &Request) -> Response<E::Body> {
    if *req.method() != Method::Get && *req.method() != Method::Head
        && *req.method() != Method::Options
    {
        return method_not_allowed::<E>();
    }

    // PARSE RANGE HEADER

    let range = range::parse_range_header(req.headers().get::<header::Range>(), entity.len());

    if let RequestedRange::NotSatisfiable = range {
        return invalid_range::<E>(entity.len());
    };

    // HANDLE CACHING HEADERS

    let resource_etag = entity.etag().unwrap();

    {
        let is_not_modified =
            if !negotiation::none_match(req.headers().get::<header::IfNoneMatch>(), &resource_etag)
            {
                true
            } else if let (Some(m), Some(&header::IfModifiedSince(since))) =
                (entity.last_modified(), req.headers().get())
            {
                m <= since
            } else {
                false
            };

        if is_not_modified {
            return not_modified::<E>(resource_etag);
        }
    }

    {
        let is_precondition_failed =
            if !negotiation::any_match(req.headers().get::<header::IfMatch>(), &resource_etag) {
                true
            } else if let (Some(m), Some(&header::IfUnmodifiedSince(since))) =
                (entity.last_modified(), req.headers().get())
            {
                m > since
            } else {
                false
            };

        if is_precondition_failed {
            return precondition_failed::<E>();
        }
    }

    let mut res = Response::new();

    res.headers_mut().set(header::ETag(resource_etag));
    res.headers_mut()
        .set(header::AcceptRanges(vec![header::RangeUnit::Bytes]));
    res.headers_mut()
        .set(header::LastModified(entity.last_modified().unwrap()));
    res.headers_mut()
        .set(header::ContentType(entity.content_type().clone()));

    // TODO: Make max-age configurable
    let max_age = time::Duration::from_secs(0);
    res.headers_mut().set(header::CacheControl(vec![
        header::CacheDirective::Public,
        header::CacheDirective::MaxAge(max_age.as_secs() as u32),
    ]));

    let compression: Option<Encoding> = if entity.len() > 1400 && mime::is_compressible_path(&path)
    {
        negotiation::negotiate_encoding(req.headers().get::<header::AcceptEncoding>())
    } else {
        None
    };

    if compression.is_some() {
        res.headers_mut()
            .set(header::ContentEncoding(vec![Encoding::Gzip]));
    }

    let body = {
        let range = match range {
            RequestedRange::Satisfiable(mut range) => {
                res.set_status(StatusCode::PartialContent);
                res.headers_mut()
                    .set(header::ContentRange(header::ContentRangeSpec::Bytes {
                        range: Some((range.start, range.end)),
                        instance_length: Some(entity.len()),
                    }));

                // NOTE: Range header is end-inclusive but std::ops::Range is end-exclusive.
                range.end += 1;

                range
            }
            _ => 0..entity.len(),
        };

        entity.get_range(range, compression)
    };

    // For HEAD requests, we do all the work except sending the body.
    if *req.method() == Method::Head {
        return res;
    }

    // TODO: It would be cleaner to do gzip here, but I couldn't figure out how to
    // satisfy the type-checker.

    res.with_body(body)
}

impl hyper::server::Service for HttpService {
    type Request = Request;
    type Response = Response<Box<Stream<Item = Vec<u8>, Error = hyper::Error> + Send>>;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let ctx = self.0;

        let work = move || match get_resource_path(&ctx.root, req.uri().path()) {
            None => Ok(Response::new().with_status(StatusCode::NotFound)),
            Some(path) => Ok(std::fs::File::open(path.clone())
                .and_then(|file| {
                    let pool = ctx.pool.clone();
                    let mime = mime::guess_mime_by_path(path.as_path());
                    ChunkedFile::new(file, pool, mime)
                })
                .map(|entity| handler(path, entity, &req))
                .unwrap_or_else(|_| Response::new().with_status(StatusCode::NotFound))),
        };

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
    if !Path::new(req_path).components().all(|c| match c {
        Component::Normal(_) | Component::RootDir =>
            true,
        _ => // e.g. neither ParentDir nor CurDir allowed.
            false,
    }) {
        return None;
    };

    let mut final_path = root.to_path_buf();
    final_path.push(&req_path[1..]);

    Some(final_path)
}

// CANNED RESPONSES

fn precondition_failed<E: Entity>() -> Response<E::Body> {
    Response::new()
        .with_status(StatusCode::PreconditionFailed)
        .with_header(header::ContentLength(0))
}

fn not_modified<E: Entity>(etag: header::EntityTag) -> Response<E::Body> {
    Response::new()
        .with_status(StatusCode::NotModified)
        .with_header(header::ETag(etag)) // Required in 304 response
        .with_header(header::ContentLength(0))
}

// TODO: Is OPTIONS part of MethodNotAllowed?
fn method_not_allowed<E: Entity>() -> Response<E::Body> {
    let body: Box<Stream<Item = E::Chunk, Error = hyper::Error> + Send> = Box::new(stream::once(
        Ok(b"This resource only supports GET, HEAD, and OPTIONS."[..].into()),
    ));
    Response::new()
        .with_status(StatusCode::MethodNotAllowed)
        .with_header(header::ContentType::plaintext())
        .with_header(header::Allow(vec![
            Method::Get,
            Method::Head,
            Method::Options,
        ]))
        .with_body(body)
}

fn invalid_range<E: Entity>(resource_len: u64) -> Response<E::Body> {
    let message = b"Invalid range";
    let body: Box<Stream<Item = E::Chunk, Error = hyper::Error> + Send> =
        Box::new(stream::once(Ok(message[..].into())));
    Response::new()
        .with_status(StatusCode::RangeNotSatisfiable)
        .with_header(header::ContentRange(header::ContentRangeSpec::Bytes {
            range: None,
            instance_length: Some(resource_len),
        }))
        .with_header(header::ContentType::plaintext())
        .with_header(header::ContentLength(message.len() as u64))
        .with_body(body)
}
