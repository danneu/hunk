use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{SocketAddr, IpAddr};

use futures_cpupool::CpuPool;
use tokio_core::reactor::Core;
use futures::{Stream};
use futures::{future::{ok}, Future};
use hyper::{self, Request, Response, server::{Http, Service}, header, Uri, Client, client::HttpConnector};
use url::Url;
use unicase::Ascii;
use std::collections::HashSet;

use config::{Site, Config};
use response;
use host::Host;
use service;

pub struct Root {
    pub config: &'static Config,
    pub pool: &'static CpuPool,
    pub remote_ip: IpAddr,
    pub client: &'static Client<HttpConnector>,
    pub sites: &'static HashMap<Host, Site>,
    pub handle: &'static ::tokio_core::reactor::Handle,
}

impl Service for Root {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        // Client must send Host header.
        // https://tools.ietf.org/html/draft-ietf-httpbis-p1-messaging-14#section-9.4
        if req.headers().get::<header::Host>().is_none() {
            return Box::new(ok(response::bad_request("missing host header")))
        }

        let req = fix_host_header(req);

        let site = req
            .headers()
            .get::<header::Host>()
            .map(|header| Host::from(header.clone()))
            .and_then(|host| self.sites.get(&host));

        let site = match site {
            Some(x) =>
                x,
            None =>
                return Box::new(ok(response::not_found())),
        };

        let next = service::log::Log {
            config: self.config,
            pool: self.pool,
            client: self.client,
            remote_ip: self.remote_ip,
            handle: self.handle,
        };

        Box::new(next.call((site, req)).map(|mut res| {
            res.headers_mut().set(header::Server::new("prox"));
            res
        }))
    }
}

/// If the request path is absolute, then the Host header is replaced with it.
///
/// <https://tools.ietf.org/html/draft-ietf-httpbis-p1-messaging-14#section-9.4>
///
///     echo -ne 'GET http://localhost:3000/a HTTP/1.1\r\nHost: example.com\r\nContent-Length: 5\r\n\r\nHello' | nc localhost 3000
fn fix_host_header(mut req: Request) -> Request {
    if !req.uri().is_absolute() {
        return req
    }

    let new_host = match req.uri().host() {
        Some(host) =>
            header::Host::new(host.to_string(), req.uri().port()),
        None =>
            return req,
    };

    req.headers_mut().set(new_host);
    req
}


#[test]
fn test_fix_host_header() {
    let mut req = Request::new(hyper::Method::Get, "http://example.com:3333".parse::<Uri>().unwrap());
    req.headers_mut().set(header::Host::new("localhost", Some(80)));
    let req2 = fix_host_header(req);
    assert_eq!(req2.headers().get::<header::Host>(), Some(&header::Host::new("example.com", Some(3333))))
}