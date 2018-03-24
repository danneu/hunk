use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{SocketAddr, IpAddr};

use futures_cpupool::CpuPool;
use chrono::prelude::Utc;
use tokio_core::reactor::Core;
use futures::{Sink};
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

pub struct Log {
    pub config: &'static Config,
    pub pool: &'static CpuPool,
    // For downstream,
    pub client: &'static Client<HttpConnector>,
    pub remote_ip: IpAddr,
    pub handle: &'static ::tokio_core::reactor::Handle,

}

impl Service for Log {
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
            service::gzip::Gzip {
                config,
                pool,
                client,
                remote_ip,
                handle,
            }
        };

        // Short-circuit if logging is disabled
        let opts = match site.log {
            None =>
                return Box::new(next().call((site, req))),
            Some(ref opts) =>
                opts,
        };

        // TODO: Figure out a way to avoid cloning the request
        let req2 = clone_req(&req);

        Box::new(next().call((site, req)).map(move |res| {
            log(remote_ip, opts, &req2, &res);
            res
        }))
    }
}

fn clone_req(src: &Request) -> Request {
    let mut req = Request::new(src.method().clone(), src.uri().clone());
    req.headers_mut().extend(src.headers().iter());
    req
}

pub fn log(remote_ip: ::std::net::IpAddr, opts: &config::Log, req: &Request, res: &Response) {
    let now = Utc::now();
//    let remote_port = peer.map(|addr| addr.port());
//    let remote_host = peer.map(|addr| addr.ip());
    let remote_host = Some(remote_ip);
    let method = format!("{}", req.method());
    let path = req.path();
    let query = req.query().unwrap_or_else(|| "");
    let url = if query.is_empty() {
        format!("{}", path)
    } else {
        format!("{}?{}", path, query)
    };
    let proto = format!("{}", req.version());
    let status = format!("{}", res.status().as_u16());

    // TODO: Send actual transferred byte count somehow, not entity length
    let bytes_tx = if let Some(&header::ContentLength(ref n)) = res.headers().get() { *n } else { 0 };

    let line = opts.format
        .replace(":remote_host", &remote_host.map(|x| format!("{}", x)) .unwrap_or_else(|| "".to_string()))
//        .replace(":remote_port", &remote_port .map(|x| format!("{}", x)) .unwrap_or_else (|| "".to_string()))
        .replace(":date_clf", &format!("{}", now.format(date_formats::CLF)))
        .replace(":date_iso8601", &format!("{}", now.format(date_formats::ISO_8601_UTC)))
        .replace(":method", &method)
        .replace(":path", path)
        .replace(":url", &url)
        .replace(":proto", &proto)
        .replace(":status", &status)
        .replace(":bytes_tx", &format!("{}", bytes_tx));

//    match opts.output {
//        Output::Stdout => println!("{}", line),
//    }

    println!("{}", line)
}

pub static COMMON_LOG_FORMAT: &'static str =
    ":remote_host - - [:date_clf] \":method :url :proto\" :status :bytes_tx";

#[allow(dead_code)]
mod date_formats {
    pub static CLF: &'static str = "%d/%b/%Y:%H:%M:%S %z";
    // ISO-8601, e.g. javascript's new Date().toISOString()
    pub static ISO_8601_UTC: &'static str = "%Y-%m-%dT%H:%M:%S%.3fZ";
    // When offset from UTC != 0, then the offset is displayed instead of "Z".
    pub static ISO_8601_OFFSET: &'static str = "%Y-%m-%dT%H:%M:%S%.3f%:z";
}

