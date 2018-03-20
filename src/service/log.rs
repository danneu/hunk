use chrono::prelude::Utc;
use futures::{Future};
use hyper::{Request, Response, header, server::Service};
use std::net::SocketAddr;

use config::Log as Config;

// TODO: Clean up messy module.

#[derive(Debug)]
pub struct Log<T> {
    peer: Option<SocketAddr>,
    config: &'static Option<Config>,
    next: T,
}

impl<T> Log<T> {
    pub fn new(peer: Option<SocketAddr>, config: &'static Option<Config>, next: T) -> Self where T: Service + 'static {
        Log { peer, config, next }
    }
}

impl<T> Service for Log<T> where T: Service<Request = Request, Response = Response> + 'static {
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

        // TODO: Find a way to factor out the clone.
        let req2 = clone_req(&req);
        let peer = self.peer;

        Box::new(self.next.call(req).map(move |res| {
            log(peer, config, &req2, &res);
            res
        }))
    }
}

fn clone_req(src: &Request) -> Request {
    let mut req = Request::new(src.method().clone(), src.uri().clone());
    req.headers_mut().extend(src.headers().iter());
    req
}

pub fn log(peer: Option<::std::net::SocketAddr>, opts: &Config, req: &Request, res: &Response) {
    let now = Utc::now();
    let remote_port = peer.map(|addr| addr.port());
    let remote_host = peer.map(|addr| addr.ip());
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
        .replace(":remote_host", &remote_host .map(|x| format!("{}", x)) .unwrap_or_else(|| "".to_string()))
        .replace(":remote_port", &remote_port .map(|x| format!("{}", x)) .unwrap_or_else (|| "".to_string()))
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

//impl Service for Log<Root> {
//    type Request = <Root as Service>::Request;
//    type Response = <Root as Service>::Response;
//    type Error = <Root as Service>::Error;
//    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;
//
//    fn call(&self, req: Self::Request) -> Self::Future {
//        println!("[log] request coming in {} {}", req.method(), req.path());
//        Box::new(self.next.call(req).map(move |res| {
////            println!("[log] response going out {} {}", req.method(), req.path());
//            res
//        }))
//    }
//}
