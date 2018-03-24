//#![allow(warnings)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_macros)]
#![feature(macro_at_most_once_rep)]
#![feature(conservative_impl_trait)]
#![feature(option_filter)]
#![feature(nll)]

//! Prox is a lightweight reverse proxy and asset server.

extern crate tokio;
#[macro_use]
extern crate hyper;
extern crate atty;
extern crate chrono;
extern crate flate2;
extern crate futures;
extern crate leak;
extern crate percent_encoding;
extern crate tokio_core;
extern crate url;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate futures_cpupool;
#[macro_use]
extern crate lazy_static;
extern crate colored;
extern crate serde;
extern crate unicase;
#[macro_use]
extern crate serde_derive;
extern crate toml;

#[macro_use]
mod util;
mod boot_message;
mod config;
mod entity;
mod etag;
mod hop;
mod host;
mod mime;
mod negotiation;
mod path;
mod range;
mod response;
mod server;
mod service;

pub use config::{Browse, Config, Gzip, Log, Server, Site, Timeouts};

pub use server::serve;
