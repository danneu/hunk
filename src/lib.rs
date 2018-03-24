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
mod server;

pub use config::Config;
pub use server::serve;

