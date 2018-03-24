use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{SocketAddr, IpAddr};

use futures_cpupool::CpuPool;
use tokio_core::reactor::Core;
use futures::{Stream};
use futures::{future::ok, Future};
use hyper::{self, Request, Response, server::{Http, Service}, header, Uri, Client, client::HttpConnector};
use url::Url;
use unicase::Ascii;
use std::collections::HashSet;

use config::Origin;
use response;
use host::Host;
use hop;

header! {
    (XForwardedFor, "X-Forwarded-For") => (IpAddr)+
}

pub struct Proxy {
    pub client: &'static Client<HttpConnector>,
    pub remote_ip: IpAddr,
}

fn without_hop_headers(headers: &header::Headers) -> header::Headers {
    headers.iter().filter(|h| !hop::is_hop_header(h.name())).collect()
}

fn make_proxy_request(mut req: Request, uri: Uri, remote_ip: IpAddr) -> Request {
    req.set_uri(uri);

    *req.headers_mut() = without_hop_headers(req.headers());

    // Update forwarded-for header
    match req.headers_mut().get_mut::<XForwardedFor>() {
        Some(ips) =>
            ips.push(remote_ip),
        None =>
            req.headers_mut().set(XForwardedFor(vec![remote_ip])),
    }

    req
}

fn make_proxy_response(mut res: Response) -> Response {
    *res.headers_mut() = without_hop_headers(res.headers());
    res
}

impl Service for Proxy {
    type Request = (&'static Origin, Request);
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, (origin, req): Self::Request) -> Self::Future {
        // Proxy only enabled if vhost.url is given.
        let dest_url = match origin.url {
            None =>
                return Box::new(ok(response::not_found())),
            Some(ref url) =>
                url
        };

        let uri = dest_url.join(&req.path()).ok()
            .and_then(|url| url.to_string().parse::<Uri>().ok());

        let uri = match uri {
            Some(x) =>
                x,
            None =>
                return Box::new(ok(response::not_found())),
        };

        let proxy_req = make_proxy_request(req, uri, self.remote_ip);
        debug!("proxy_req: {:#?}", proxy_req);

        let future = self.client.request(proxy_req)
            .then(|res| match res {
                Ok(res) =>
                    Ok(make_proxy_response(res)),
                Err(_) =>
                    Ok(response::internal_server_error()),
            });

        Box::new(future)
    }
}