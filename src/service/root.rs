use futures_cpupool::CpuPool;
use futures::{Future};
use hyper::server::{Request, Response, Service};
use hyper::{self, Body, Method, header, StatusCode};

use std::fs::File;

use response;
use entity;
use mime;
use range;
use negotiation;
use config;
use path;

const CHUNK_SIZE: u64 = 65_536;

#[derive(Debug)]
pub struct Root {
    pool: &'static CpuPool,
    config: &'static config::Server,
}

impl Root {
    pub fn new(pool: &'static CpuPool, config: &'static config::Server) -> Self {
        Root { pool, config }
    }
}

impl Service for Root {
    type Request = Request;
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let pool = self.pool.clone();
        let config = self.config;

        Box::new(self.pool.spawn_fn(move || {
            let res = handle_request(pool, config, &req);
            Ok(res)
        }))
    }
}

fn handle_request(pool: CpuPool, config: &'static config::Server, req: &Request) -> Response<Body> {
    if *req.method() != Method::Get && *req.method() != Method::Head && *req.method() != Method::Options {
        return response::method_not_allowed();
    }

    let entity_path = match path::get_entity_path(&config.root, req.path()) {
        None => return response::not_found(),
        Some(path) => path,
    };

    let file = match File::open(&entity_path) {
        Err(_) => return response::not_found(),
        Ok(file) => file,
    };

    let entity = match entity::Entity::new(
        file,
        pool.clone(),
        mime::guess_mime_by_path(&entity_path),
    ) {
        Err(_) => return response::not_found(),
        Ok(entity) => entity,
    };

    let mut res = Response::new();

    // HANDLE CACHING HEADERS

    let entity_etag = entity.etag(&entity::ETagKind::Strong);

    if is_not_modified(&entity, req, &entity_etag) {
        return response::not_modified(entity_etag);
    }

    if is_precondition_failed(&entity, req, &entity_etag) {
        return response::precondition_failed();
    }

    // PARSE RANGE HEADER
    // - Comes after evaluating precondition headers.
    //   <https://tools.ietf.org/html/rfc7233#section-3.1>

    let range = range::parse_range_header(
        req.headers().has::<header::Range>(),
        req.headers().get::<header::Range>(),
        entity.len(),
    );

    // Client provided a bad range
    if let range::RequestedRange::NotSatisfiable = range {
        return response::invalid_range(entity.len());
    };

    // COMMON HEADERS

    res.headers_mut().set(header::ETag(entity_etag));
    res.headers_mut().set(header::AcceptRanges(vec![header::RangeUnit::Bytes]));
    res.headers_mut().set(header::LastModified(entity.last_modified()));
    res.headers_mut().set(header::ContentType(entity.content_type().mime.clone()));

    // More about Content-Length: <https://tools.ietf.org/html/rfc2616#section-4.4>
    // - Represents length *after* transfer-encoding.
    // - Don't set Content-Length if Transfer-Encoding != 'identity'
    res.headers_mut().set(header::ContentLength(entity.len()));

    // Only set max-age if it's configured at all.
//    if let Some(max_age) = ctx.config.cache.as_ref().map(|opts| opts.max_age) {
//        res.headers_mut().set(header::CacheControl(vec![
//            header::CacheDirective::Public,
//            header::CacheDirective::MaxAge(max_age.as_secs() as u32),
//        ]));
//    }

    // Start streaming the file.

    let body = {
        let range = match range {
            range::RequestedRange::Satisfiable(mut range) => {
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

        entity.get_range(range, CHUNK_SIZE)
    };

    // For HEAD requests, we do all the work except sending the body.
    if *req.method() == Method::Head {
        return res;
    }

    res.with_body(body)
}

fn is_not_modified(
    entity: &entity::Entity,
    req: &Request,
    entity_etag: &header::EntityTag,
) -> bool {
    if !negotiation::none_match(req.headers().get::<header::IfNoneMatch>(), entity_etag) {
        true
    } else if let Some(&header::IfModifiedSince(since)) = req.headers().get() {
        entity.last_modified() <= since
    } else {
        false
    }
}

fn is_precondition_failed(
    entity: &entity::Entity,
    req: &Request,
    entity_etag: &header::EntityTag,
) -> bool {
    if !negotiation::any_match(req.headers().get::<header::IfMatch>(), entity_etag) {
        true
    } else if let Some(&header::IfUnmodifiedSince(since)) = req.headers().get() {
        entity.last_modified() > since
    } else {
        false
    }
}
