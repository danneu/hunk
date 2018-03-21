//#![allow(warnings)]

#![feature(conservative_impl_trait)]
#![feature(macro_at_most_once_rep)]
#![feature(nll)]
#![feature(pattern_parentheses)]
#![feature(option_filter)]
#![feature(proc_macro)] // For maud

extern crate url;
extern crate tokio;
extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate flate2;
extern crate hyper;
extern crate maud;
extern crate atty;
extern crate unicase;
extern crate colored;
extern crate chrono;
extern crate leak;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate percent_encoding;
#[macro_use] extern crate lazy_static;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate toml;
extern crate regex;

use futures_cpupool::CpuPool;
use futures::{future::{Executor}, Future};
use futures::{Stream};
use hyper::{Chunk};
use hyper::server::{Http};
use tokio_core::reactor::Core;
use tokio::net::TcpListener;
use leak::Leak;

use std::net::SocketAddr;

mod path;
mod service;
mod response;
mod compress;
mod base36;
mod negotiation;
mod range;
#[macro_use] mod util;
mod entity;
mod mime;
mod config_print;
mod config;

pub use config::Config;

pub fn serve(config: Config) {
    env_logger::init();

    use service::{log::Log, cors::Cors, root::Root, compress::Compress, browse::Browse, gate::Gate};

    let pool = Box::new(CpuPool::new(1)).leak();

    let config = Box::new(config).leak();

    // For Browse middleware.
    let root = Box::new(config.server.root.clone()).leak();

    let factory = move |peer: Option<SocketAddr>| {
        // Request travels from bottom to top,
        // Response travels from top to bottom.
        pipe!(
            Root::new(pool, &config.server),
            (Browse::new[&config.browse, root.as_path()]),
            (Cors::new[&config.cors]),
            (Compress::new[pool, &config.gzip]),
            (Log::new[peer, &config.log]),
            (Gate::new[])
        )
    };

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut http: Http<Chunk> = Http::new();
    http.sleep_on_errors(true);

    let listener = TcpListener::bind(&config.server.addr).unwrap();
    let server = listener.incoming().for_each(|tcp| {
        let peer = tcp.peer_addr().ok();
        let conn = http.serve_connection(tcp, factory(peer))
            .map(|_| ())
            .map_err(|_e| {
                // Note: Noisy (epipe)
                // error!("http.serve_connection error: {:?}", _e);
                ()
            });

        handle.execute(conn)
            .map(|_| ())
            .map_err(|e| {
                error!("handle.execute error: {:?}", e);
                // TODO: Figure out how to handle this.
                // For now, just unify with expected io::Error
                std::io::Error::new(std::io::ErrorKind::Other, "error stub")
            })
    });

    if atty::is(atty::Stream::Stdout) {
        config_print::pretty(config);
    } else {
        info!("listening at {}", config.server.addr);
    }

    core.run(server).unwrap();
}
