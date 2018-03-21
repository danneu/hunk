use futures::{Future};
use hyper::server::{Request, Response, Service};
use hyper::{header, Method};
use flate2::Compression;
use unicase::Ascii;

use negotiation;
use mime;
use compress;
use util;
use config;

#[derive(Debug)]
pub struct Compress<T> {
    pool: &'static ::futures_cpupool::CpuPool,
    config: &'static Option<config::Gzip>,
    next: T,
}

impl<T> Compress<T> {
    pub fn new(pool: &'static ::futures_cpupool::CpuPool, config: &'static Option<config::Gzip>, next: T) -> Self where T: Service + 'static {
        Compress { pool, config, next }
    }
}

impl<T> Service for Compress<T>
    where T: Service<Request = Request, Response = Response> + 'static
{
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let config = match *self.config {
            None =>
                return Box::new(self.next.call(req)),
            Some(ref config) =>
                config
        };

        // Only compress GET and HEAD
        if *req.method() != Method::Get && *req.method() != Method::Head {
            return Box::new(self.next.call(req))
        }

        let pool = self.pool.clone();
        let req_accept_encoding = req.headers().get::<header::AcceptEncoding>().cloned();


        Box::new(self.next.call(req).map(move |mut res| {
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
            let content_length = match res.headers().get::<header::ContentLength>() {
                None =>
                    return res,
                Some(&header::ContentLength(length)) =>
                    length,
            };

            let should_compress = mime::is_mime_compressible(mime) && content_length >= config.threshold && {
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
                .with_body(compress::gzip(&pool, Compression::new(1), res.body()))
        }))
    }
}
