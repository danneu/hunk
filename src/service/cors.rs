use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{SocketAddr, IpAddr};

use futures_cpupool::CpuPool;
use chrono::prelude::Utc;
use tokio_core::reactor::Core;
use futures::{Sink};
use futures::{future::ok, Future};
use hyper::{self, Request, Response, server::{Http, Service}, header, Uri, Client, client::HttpConnector, Method, Body};
use url::Url;
use unicase::Ascii;
use std::collections::HashSet;
use flate2;

use config::{self, Config, Site, CorsOrigin};
use response;
use host::Host;
use hop;
use service;
use mime;
use negotiation;
use util;

pub struct Cors {
    pub config: &'static Config,
    pub pool: &'static CpuPool,
    // For downstream,
    pub client: &'static Client<HttpConnector>,
    pub remote_ip: IpAddr,
    pub handle: &'static ::tokio_core::reactor::Handle,
}

impl Service for Cors {
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
            service::browse::Browse {
                config,
                pool,
                client,
                remote_ip,
                handle,
            }
        };

        // Short-circuit if logging is disabled
        let config: &config::Cors = match site.cors {
            None =>
                return Box::new(next().call((site, req))),
            Some(ref opts) =>
                opts,
        };

        // Bail if request has no Origin header
        let req_origin: header::Origin = match req.headers().get::<header::Origin>() {
            None =>
                return Box::new(next().call((site, req))),
            Some(site) =>
                site.clone(),
        };

        let allow_origin = match config.origin {
            CorsOrigin::Any =>
                true,
            CorsOrigin::Few(ref alloweds) =>
                alloweds.contains(&req_origin),
        };

        if *req.method() == Method::Options {
            let mut res = Response::new();
            util::append_header_vary(&mut res.headers_mut(), Ascii::new("Origin".to_string()));
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
            Box::new(next().call((site, req)).map(move |mut res: Response| {
                // Bail if Origin does not match our allowed set
                if !allow_origin {
                    return res
                }

                // Always set Vary header if CORS is enabled.
                util::append_header_vary(&mut res.headers_mut(), Ascii::new("Origin".to_string()));

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

