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

use config::{Origin, Config};
use response;
use host::Host;
use service;

pub struct Root {
    pub config: &'static Config,
    pub pool: &'static CpuPool,
    pub remote_ip: IpAddr,
    pub client: &'static Client<HttpConnector>,
    pub origins: &'static HashMap<Host, Origin>,
}

impl Service for Root {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let origin = req
            .headers()
            .get::<header::Host>()
            .map(|header| Host::from(header.clone()))
            .and_then(|host| self.origins.get(&host));

        let origin = match origin {
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
        };

        Box::new(next.call((origin, req)).map(|mut res| {
            res.headers_mut().set(header::Server::new("Hunk"));
            res
        }))
    }
}