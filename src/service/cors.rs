use futures::{future::ok, Future};
use hyper::server::{Request, Response, Service};
use hyper::{self, header, Method};
use unicase::Ascii;

use util;
use config::Cors as Config;
use config::Origin;

// CORS
// https://www.w3.org/TR/cors/#resource-processing-model
// NOTE: The string "*" cannot be used for a resource that supports credentials.

#[derive(Debug)]
pub struct Cors<T> {
    config: &'static Option<Config>,
    next: T,
}

impl<T> Cors<T> {
    pub fn new(config: &'static Option<Config>, next: T) -> Self {
        Cors { config, next }
    }
}

impl<T> Service for Cors<T> where T: Service<Request = Request, Response = Response> + 'static {
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

        // Bail if request has no Origin header
        let req_origin: header::Origin = match req.headers().get::<header::Origin>() {
            None =>
                return Box::new(self.next.call(req)),
            Some(origin) =>
                origin.clone(),
        };

        let allow_origin = match config.origin {
            Origin::Any =>
                true,
            Origin::Few(ref alloweds) =>
                alloweds.contains(&req_origin),
        };

        if *req.method() == Method::Options {
            let mut res = Response::new();
            util::append_header_vary(&mut res, Ascii::new("Origin".to_string()));
            res.headers_mut().set(header::ContentLength(0));
            res.headers_mut().set(header::ContentType(hyper::mime::TEXT_PLAIN_UTF_8));

            // Bail if Origin does not match our allowed set
            // Notice that preflight branch bails with its new response.
            // Non-preflight branch bails with the downstream response.
            if !allow_origin {
                return Box::new(ok(res))
            }

            res.headers_mut().set(header::AccessControlAllowMethods(
                config.methods.iter().cloned().collect()
            ));

            let actual_method = match req.headers().get::<header::AccessControlRequestMethod>() {
                // Bail if no method given
                None => return Box::new(ok(res)),
                Some(method) => method,
            };

            // Bail if unapproved method
            if !config.methods.contains(actual_method) {
                return Box::new(ok(res))
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
                .any(|k| !config.allowed_headers.contains(k))
                {
                    return Box::new(ok(res))
                }

            // Success, so set the allow origin header.
            res.headers_mut().set(header::AccessControlAllowOrigin::Value(format!("{}", req_origin)));

            if config.allow_credentials {
                res.headers_mut().set(header::AccessControlAllowCredentials);
            }

            if let Some(max_age) = config.max_age {
                res.headers_mut().set(header::AccessControlMaxAge(max_age));
            }

            Box::new(ok(res))
        } else {
            Box::new(self.next.call(req).map(move |mut res: Response| {
                // Bail if Origin does not match our allowed set
                if !allow_origin {
                    return res
                }

                // Always set Vary header if CORS is enabled.
                util::append_header_vary(&mut res, Ascii::new("Origin".to_string()));

                res.headers_mut().set(header::AccessControlAllowOrigin::Value(format!("{}", req_origin)));

                if config.allow_credentials {
                    res.headers_mut().set(header::AccessControlAllowCredentials);
                }

                if !config.exposed_headers.is_empty() {
                    res.headers_mut().set(header::AccessControlExposeHeaders(
                        config.exposed_headers.clone()
                    ))
                }

                res
            }))
        }

    }
}
