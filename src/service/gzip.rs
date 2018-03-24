use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{Hash};
use std::net::{SocketAddr, IpAddr};

use futures_cpupool::CpuPool;
use tokio_core::reactor::Core;
use futures::{Sink, Stream};
use futures::{future, Future};
use hyper::{self, Request, Response, server::{Http, Service}, header, Uri, Client, client::HttpConnector, Method, Body};
use url::Url;
use unicase::Ascii;
use std::collections::HashSet;
use flate2;

use config::{self, Config, Site};
use response;
use host::Host;
use hop;
use service;
use mime;
use negotiation;
use util;

pub struct Gzip {
    pub config: &'static Config,
    pub pool: &'static CpuPool,
    // For downstream,
    pub client: &'static Client<HttpConnector>,
    pub remote_ip: IpAddr,
    pub handle: &'static ::tokio_core::reactor::Handle,
}

impl Service for Gzip {
    type Request = (&'static Site, Request);
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, (site, req): Self::Request) -> Self::Future {
        let config = self.config;
        let pool = self.pool;
        let client = self.client;
        let remote_ip = self.remote_ip;
        let handle = self.handle;

        let next = move || {
            service::cors::Cors {
                config,
                pool,
                client,
                remote_ip,
                handle,
            }
        };

        let opts = match site.gzip {
            None =>
                return Box::new(next().call((site, req))),
            Some(ref opts) =>
                opts
        };

        // Only compress GET and HEAD
        if *req.method() != Method::Get && *req.method() != Method::Head {
            return Box::new(next().call((site, req)))
        }

        let req_accept_encoding = req.headers().get::<header::AcceptEncoding>().cloned();

        Box::new(next().call((site, req)).map(move |res| {
            handle_response(pool, res, opts, req_accept_encoding)
        }))
    }
}

fn handle_response(pool: &CpuPool, mut res: Response, opts: &config::Gzip, req_accept_encoding: Option<header::AcceptEncoding>) -> Response {
    // Only compress if successful response
    if !res.status().is_success() {
        return res
    }

    let mime = match res.headers().get::<header::ContentType>() {
        None =>
            return res,
        Some(&header::ContentType(ref mime)) =>
            mime,
    };

    // Content-Length is always set by Root service
    // TODO: Re-check this old comment
    let content_length = match res.headers().get::<header::ContentLength>() {
        None =>
            return res,
        Some(&header::ContentLength(length)) =>
            length,
    };

    let should_compress = mime::is_mime_compressible(mime) && content_length >= opts.threshold && {
        let encoding = negotiation::negotiate_encoding(req_accept_encoding.as_ref());
        encoding == Some(header::Encoding::Gzip)
    };

    if !should_compress {
        return res
    }

    // Remove Content-Length
    res.headers_mut().set(header::TransferEncoding(vec![header::Encoding::Chunked]));
    res.headers_mut().remove::<header::ContentLength>();

    // Set Content-Encoding
    res.headers_mut().set(header::ContentEncoding(vec![header::Encoding::Gzip]));

    // Append Accept-Encoding
    util::append_header_vary(&mut res.headers_mut(), Ascii::new("Accept-Encoding".to_string()));

    // Weaken ETag
    if let Some(etag) = res.headers().get::<header::ETag>() {
        if !etag.weak {
            let etag = header::EntityTag::weak(etag.tag().to_string());
            res.headers_mut().set::<header::ETag>(header::ETag(etag))
        }
    }

    // TODO: Would be nicer to have something like:
    //
    //     let (head, body) = response.split();
    //     let response = Response::join(head, transform(body))
    Response::new()
        .with_status(res.status())
        .with_headers(res.headers().clone())
        .with_body(gzip(&pool, flate2::Compression::new(1), res.body()))
}

/// Transforms the body into a new one where all of its chunks will be gzipped.
//
// TODO: Convert back to Stream -> Stream transformation when Hyper 0.12.x releases
// since it'll have Body::wrap_stream().
pub fn gzip(pool: &CpuPool, level: flate2::Compression, body: Body) -> Body {
    use flate2::{write::GzEncoder};
    use std::io::Write;

    let stream = body.and_then(move |chunk| {
        let mut encoder = GzEncoder::new(Vec::new(), level);
        encoder
            .write_all(&chunk)
            .and_then(|_| encoder.finish())
            .map(|vec| vec.into())
            .map_err(|e| e.into())
    });

    let (tx, body) = Body::pair();

    pool.spawn(tx.send_all(stream.map(Ok).map_err(|_| unreachable!())))
        .forget();

    body
}
