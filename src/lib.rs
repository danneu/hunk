#![feature(ip_constructors)]

extern crate flate2;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate unicase;
extern crate chrono;

// 3rd party

use futures::{stream, Future};
use futures_cpupool::CpuPool;

use hyper::{Method, StatusCode};
use hyper::server::{Request, Response};
use hyper::header;

use unicase::Ascii;

use chrono::format::strftime;
use chrono::prelude::Utc;
use chrono::prelude::Local;

use std::fs::File;
use std::path::{self, Path, PathBuf};
use std::collections::HashSet;
use std::time;

// 1st party

mod range;
mod negotiation;
mod mime;
mod base36;
mod util;
mod chunks;
mod resource;
pub mod logger;
pub mod config;
pub mod options;

use range::RequestedRange;
use chunks::ChunkStream;
use resource::Resource;

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

lazy_static! {
    static ref SIMPLE_CORS_HEADERS: HashSet<Ascii<String>> = {
        let mut x = HashSet::new();
        x.insert(Ascii::new("Cache-Control".to_string()));
        x.insert(Ascii::new("Content-Language".to_string()));
        x.insert(Ascii::new("Content-Type".to_string()));
        x.insert(Ascii::new("Expires".to_string()));
        x.insert(Ascii::new("Last-Modified".to_string()));
        x.insert(Ascii::new("Pragma".to_string()));
        x
    };
}

// TODO: Incomplete
//
// https://www.w3.org/TR/cors/#resource-processing-model
//
// Returns true if response is finished being handled.
fn handle_cors(
    cors: Option<&options::Cors>,
    req: &Request,
    res: &mut Response<ChunkStream>,
) -> bool {
    // Bail if user has no cors options configured
    let cors = match cors {
        None => return false,
        Some(cors) => cors,
    };

    // Bail if request has no Origin header
    let req_origin = match req.headers().get::<header::Origin>() {
        None => return false,
        Some(origin) => origin,
    };

    let allow_origin = match cors.origin {
        options::Origin::Any => true,
        options::Origin::Few(ref alloweds) => alloweds.iter().any(|allowed| allowed == req_origin),
    };

    // Bail if Origin does not match our allowed set
    if !allow_origin {
        return false;
    }

    // Now that valid Origin was given, add Vary header
    res.headers_mut()
        .set(header::Vary::Items(vec![Ascii::new("Origin".to_string())]));

    // Branch the logic between OPTIONS requests and all the rest.
    if *req.method() == Method::Options {
        res.headers_mut().set(header::ContentLength(0));
        res.headers_mut()
            .set(header::ContentType(::hyper::mime::TEXT_PLAIN_UTF_8));

        // Preflight
        let actual_method = match req.headers().get::<header::AccessControlRequestMethod>() {
            // Bail if no method given
            None => return true,
            Some(method) => method,
        };

        // Bail if unapproved method
        if !cors.methods.contains(actual_method) {
            return true;
        }

        let actual_header_keys: Vec<Ascii<String>> = match req.headers()
            .get::<header::AccessControlRequestHeaders>()
        {
            None => Vec::new(),
            Some(&header::AccessControlRequestHeaders(ref keys)) => keys.to_vec(),
        };

        // Bail if any header isn't in our approved set
        if actual_header_keys
            .iter()
            .any(|k| !cors.allowed_headers.contains(k))
        {
            return true;
        }

        // Success, so set the allow origin header.
        res.headers_mut()
            .set(header::AccessControlAllowOrigin::Value(format!(
                "{}",
                req_origin
            )));

        if cors.allow_credentials {
            res.headers_mut().set(header::AccessControlAllowCredentials);
        }

        if let Some(max_age) = cors.max_age {
            res.headers_mut().set(header::AccessControlMaxAge(max_age));
        }

        // Don't have to add these headers if method is a simple cors method.
        res.headers_mut().set(header::AccessControlAllowMethods(
            cors.methods.iter().cloned().collect()
        ));

        // These don't make much sense either since it's just a static file server, but
        // I can always remove it later.
        let nonsimple = |k: &Ascii<String>| -> bool {
            !SIMPLE_CORS_HEADERS.contains(k) || k == &Ascii::new("Content-Type".to_string())
        };
        if actual_header_keys.iter().any(nonsimple) {
            res.headers_mut().set(header::AccessControlAllowHeaders(
                cors.allowed_headers.iter().cloned().collect(),
            ))
        }

        true
    } else {
        // Non-preflight requests
        res.headers_mut()
            .set(header::AccessControlAllowOrigin::Value(format!(
                "{}",
                req_origin
            )));

        if cors.allow_credentials {
            res.headers_mut().set(header::AccessControlAllowCredentials);
        }

        if !cors.exposed_headers.is_empty() {
            res.headers_mut().set(header::AccessControlExposeHeaders(
                cors.exposed_headers.to_vec()
            ))
        }

        false
    }
}


fn handler(ctx: &'static Context, req: &Request) -> Response<ChunkStream> {
    if *req.method() != Method::Get && *req.method() != Method::Head
        && *req.method() != Method::Options
    {
        return method_not_allowed();
    }

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

    // CORS
    // https://www.w3.org/TR/cors/#resource-processing-model
    // NOTE: The string "*" cannot be used for a resource that supports credentials.

    let mut res: Response<ChunkStream> = Response::new();

    if handle_cors(ctx.opts.cors.as_ref(), req, &mut res) {
        return res;
    }

    // HANDLE CACHING HEADERS

    let should_gzip = ctx.opts
        .gzip
        .as_ref()
        .map(|opts| {
            resource.len() >= opts.threshold && resource.content_type().compressible
                && negotiation::negotiate_encoding(req.headers().get::<header::AcceptEncoding>())
                    == Some(header::Encoding::Gzip)
        })
        .unwrap_or(false);

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

    let range = if should_gzip {
        // Ignore Range if response is gzipped
        RequestedRange::None
    } else {
        range::parse_range_header(
            req.headers().has::<header::Range>(),
            req.headers().get::<header::Range>(),
            resource.len(),
        )
    };

    if let RequestedRange::NotSatisfiable = range {
        return invalid_range(resource.len());
    };

    res.headers_mut().set(header::ETag(resource_etag));
    res.headers_mut()
        .set(header::AcceptRanges(vec![header::RangeUnit::Bytes]));
    res.headers_mut()
        .set(header::LastModified(resource.last_modified()));
    res.headers_mut()
        .set(header::ContentType(resource.content_type().mime.clone()));

    // More about Content-Length: <https://tools.ietf.org/html/rfc2616#section-4.4>
    // - Represents length *after* transfer-encoding.
    // - Don't set Content-Length if Transfer-Encoding != 'identity'
    if should_gzip {
        res.headers_mut()
            .set(header::TransferEncoding(vec![header::Encoding::Chunked]));
    } else {
        res.headers_mut().set(header::ContentLength(resource.len()));
    }

    // Accept-Encoding doesn't affect the response unless gzip is turned on
    if ctx.opts.gzip.is_some() {
        res.headers_mut().set(header::Vary::Items(vec![
            unicase::Ascii::new("Accept-Encoding".to_owned()),
        ]));
    }

    // Only set max-age if it's configured at all.
    if let Some(max_age) = ctx.opts.cache.as_ref().map(|opts| opts.max_age) {
        res.headers_mut().set(header::CacheControl(vec![
            header::CacheDirective::Public,
            header::CacheDirective::MaxAge(max_age),
        ]));
    }

    let body: ChunkStream = {
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

        resource.get_range(range, CHUNK_SIZE)
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
        res.with_body(chunks::gzip(body, ctx.opts.gzip.as_ref().unwrap().level))
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

        let work = move || {
            let res = handler(ctx, &req);

            if let Some(ref log) = ctx.opts.log {
                log.logger.log(&req, &res);

            }

            Ok(res)
        };

        Box::new(ctx.pool.spawn_fn(work))
    }
}

// A path is safe if it doesn't try to /./ or /../
fn is_safe_path(path: &Path) -> bool {
    path.components().all(|c| match c {
        path::Component::RootDir | path::Component::Normal(_) => true,
        _ => false,
    })
}

// Join root with request path to get the asset path candidate.
fn get_resource_path(root: &Path, req_path: &str) -> Option<PathBuf> {
    // request path must be absolute
    if !req_path.starts_with('/') {
        return None;
    }

    // request path cannot be a directory {
    if req_path.ends_with('/') {
        return None;
    }

    // Security: request path cannot climb directories
    if !is_safe_path(Path::new(req_path)) {
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
