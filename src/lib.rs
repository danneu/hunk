//#![allow(warnings)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![feature(macro_at_most_once_rep)]
#![feature(conservative_impl_trait)]
#![feature(option_filter)]
#![feature(nll)]

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

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};

use futures::Stream;
use futures::{future, Future};
use futures_cpupool::CpuPool;
use hyper::client::FutureResponse;
use hyper::header;
use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response, Service};
use hyper::{Client, Uri, client::HttpConnector};
use leak::Leak;
use std::collections::HashSet;
use tokio_core::reactor::Core;
use unicase::Ascii;
use url::Url;

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

pub use config::Config;
pub use server::serve;
