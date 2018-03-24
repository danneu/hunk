#![allow(warnings)]

#![feature(macro_at_most_once_rep)]
#![feature(conservative_impl_trait)]
#![feature(option_filter)]
#![feature(nll)]

extern crate tokio;
#[macro_use] extern crate hyper;
extern crate chrono;
extern crate futures;
extern crate tokio_core;
extern crate atty;
extern crate url;
extern crate percent_encoding;
extern crate leak;
extern crate flate2;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate futures_cpupool;
#[macro_use]
extern crate lazy_static;
extern crate colored;
extern crate unicase;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate toml;

use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{SocketAddr, IpAddr};

use leak::Leak;
use futures_cpupool::CpuPool;
use tokio_core::reactor::Core;
use futures::{Stream};
use futures::{future, Future};
use hyper::header::ContentLength;
use hyper::header;
use hyper::server::{Http, Request, Response, Service};
use hyper::client::FutureResponse;
use hyper::{Uri, Client, client::HttpConnector};
use url::Url;
use unicase::Ascii;
use std::collections::HashSet;

#[macro_use] mod util;
mod etag;
mod path;
mod negotiation;
mod range;
mod boot_message;
mod mime;
mod hop;
mod entity;
mod config;
mod response;
mod host;
mod service;

pub use config::Config;
use host::Host;

pub fn serve(config: Config) {
    env_logger::init();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let pool = Box::new(CpuPool::new(1)).leak();
    let config = Box::new(config.clone()).leak();
    let client = Box::new(Client::new(&handle)).leak();
    let sites = {
        let mut map = HashMap::new();
        for site in config.clone().sites {
            for host in &site.host {
                map.insert(host.clone(), site.clone());
            }
        }
        Box::new(map).leak()
    };

    let mut http: Http<hyper::Chunk> = Http::new();
    http.sleep_on_errors(true);

    let listener = tokio_core::net::TcpListener::bind(&config.server.bind, &handle).unwrap();
    let factory = move |remote_ip| {
        service::root::Root {
            client,
            config,
            sites,
            remote_ip,
            pool,
        }
    };

    let future = listener.incoming().for_each(move |(socket, peer)| {
        let conn = http.serve_connection(socket, factory(peer.ip()))
            .map(|_| ())
            .map_err(|_| {
                // Note: Very noisy (epipe)
                // error!("server connection error: {}", e)
            });

        handle.spawn(conn);
        Ok(())
    });

    if atty::is(atty::Stream::Stdout) {
        boot_message::pretty(config);
    } else {
        info!("[prox] listening on {}", config.server.bind);
    }

    core.run(future).unwrap()

}
