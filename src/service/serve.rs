use std::fs::File;
use std::net::IpAddr;
use std::path::Path;

use futures::{Future, future::ok};
use futures_cpupool::CpuPool;
use hyper::{self, header, Client, Method, Request, Response, StatusCode, client::HttpConnector,
            server::Service};

use config::{self, Config, Site};
use entity;
use mime;
use negotiate;
use path;
use range;
use response;
use service;

const CHUNK_SIZE: u64 = 65_536;

pub struct Serve {
    pub config: &'static Config,
    pub pool: &'static CpuPool,
    // For downstream,
    pub client: &'static Client<HttpConnector>,
    pub remote_ip: IpAddr,
    pub handle: &'static ::tokio_core::reactor::Handle,
}

fn handle_request_sync(
    pool: CpuPool,
    root: &'static Path,
    req: Request,
    dotfiles: &bool,
) -> (Request, Option<Response>) {
    if *req.method() != Method::Get && *req.method() != Method::Head
        && *req.method() != Method::Options
    {
        //return Box::new(ok(response::method_not_allowed()));
        return (req, None);
    }

    let entity_path = match path::get_entity_path(root, req.path()) {
        None => //return Box::new(ok(response::not_found())),
            return (req, None),
        Some(path) => path,
    };

    // Short-circuit if dotfile forbidden
    if !dotfiles && entity_path.file_name().and_then(|x| x.to_str()).map(|x| x.starts_with('.')).unwrap_or(false) {
        return (req, None)
    }

    let file = match File::open(&entity_path) {
        Err(_) =>// return Box::new(ok(response::not_found())),
            return (req, None),
        Ok(file) => file,
    };

    // Only service files
    if let Ok(false) = file.metadata().map(|meta| meta.is_file()) {
        return (req, Some(response::not_found()));
    }

    let entity = match entity::Entity::new(
        file,
        pool,
        mime::guess_mime_by_path(&entity_path),
    ) {
        Err(_) => // return Box::new(ok(response::not_found())),
            return (req, None),
        Ok(entity) => entity,
    };

    let mut res = Response::new();

    // HANDLE CACHING HEADERS

    let entity_etag = entity.etag(&entity::ETagKind::Strong);

    if is_not_modified(&entity, req.headers(), &entity_etag) {
        return (req, Some(response::not_modified(entity_etag)));
    }

    if is_precondition_failed(&entity, req.headers(), &entity_etag) {
        return (req, Some(response::precondition_failed()));
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
        return (req, Some(response::invalid_range(entity.len())));
    };

    // COMMON HEADERS

    res.headers_mut().set(header::ETag(entity_etag));
    res.headers_mut()
        .set(header::AcceptRanges(vec![header::RangeUnit::Bytes]));
    res.headers_mut()
        .set(header::LastModified(entity.last_modified()));
    res.headers_mut()
        .set(header::ContentType(entity.content_type().mime.clone()));

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
        return (req, Some(res));
    }

    (req, Some(res.with_body(body)))
}

// If None, then downstream should handle.
// TODO: Think about None vs Not Found responses.
// TODO: Get pool.spawn(handle_request()) working again instead of the weird _sync helper.
fn handle_request(
    pool: &CpuPool,
    root: &'static Path,
    req: Request,
    dotfiles: &bool,
) -> impl Future<Item = (Request, Option<Response>), Error = hyper::Error> {
    pool.spawn(ok(handle_request_sync(pool.clone(), root, req, dotfiles)))
}

impl Service for Serve {
    type Request = (&'static Site, Request);
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, (site, req): Self::Request) -> Self::Future {
        let client = self.client;
        let remote_ip = self.remote_ip;
        let config = self.config;
        let handle = self.handle;

        let next = move || service::proxy::Proxy {
            client,
            remote_ip,
            config,
            handle,
        };

        // Short-circuit if serve is not set.
        let config::Serve { ref root, ref dotfiles, .. } = match &site.serve {
            Some(x) => x,
            None => return next().call((site, req)),
        };

        // See if path hits a static file.

        let future = handle_request(
            self.pool,
            root,
            req,
            dotfiles,
        );

        Box::new(future.then(move |result| match result {
            Ok((_, Some(res))) => Box::new(ok(res)),
            Ok((req, None)) => next().call((site, req)),
            Err(e) => {
                error!("io error when fetching static file: {:?}", e);
                Box::new(ok(response::internal_server_error()))
            },
        }))
    }
}

fn is_not_modified(
    entity: &entity::Entity,
    headers: &header::Headers,
    entity_etag: &header::EntityTag,
) -> bool {
    if !negotiate::none_match(headers.get::<header::IfNoneMatch>(), entity_etag) {
        true
    } else if let Some(&header::IfModifiedSince(since)) = headers.get() {
        entity.last_modified() <= since
    } else {
        false
    }
}

fn is_precondition_failed(
    entity: &entity::Entity,
    headers: &header::Headers,
    entity_etag: &header::EntityTag,
) -> bool {
    if !negotiate::any_match(headers.get::<header::IfMatch>(), entity_etag) {
        true
    } else if let Some(&header::IfUnmodifiedSince(since)) = headers.get() {
        entity.last_modified() > since
    } else {
        false
    }
}
