use std::net::SocketAddr;
use std::error::Error;
use std::path::PathBuf;
use std::iter::FromIterator;
use std::collections::HashSet;

use serde;
use regex::Regex;
use unicase::Ascii;
use hyper::{header, Method};
use url::{self, Url};

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub server: Server,
    pub gzip: Option<Gzip>,
    pub log: Option<Log>,
    pub cors: Option<Cors>,
    pub browse: Option<Browse>,
}

#[derive(Debug, Clone)]
pub struct Server {
    pub root: PathBuf,
    pub addr: SocketAddr,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            root: default_root(),
            addr: default_addr().parse().unwrap(),
        }
    }
}

fn default_root() -> PathBuf {
    PathBuf::from(".").canonicalize().unwrap()
}

fn default_addr() -> String {
    format!("127.0.0.1:{}", default_port())
}

fn default_port() -> u32 {
    3000
}

#[derive(Deserialize, Debug, Clone)]
pub struct Gzip {
    // Gzip: 0-9
    // We default 1 because it has the maximum compression to cpu ratio.
    // pub level: _,
    #[serde(default = "default_threshold")]
    pub threshold: u64,
}

fn default_threshold() -> u64 {
    1400
}

#[derive(Deserialize, Debug, Clone)]
pub struct Log {
    #[serde(default = "default_log_format")]
    pub format: String
}

fn default_log_format() -> String {
    super::service::log::COMMON_LOG_FORMAT.to_string()
}


#[derive(Deserialize, Debug, Clone)]
pub struct Browse {}

impl<'de> serde::Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize)]
        struct Http_ {
            #[serde(default = "default_root")]
            root: PathBuf,
            #[serde(default = "default_addr")]
            addr: String,
        }

        let input = Http_::deserialize(deserializer)?;

        // Transform

        // Allow localhost short-hand
        let mut addr = input.addr.replace("localhost", "127.0.0.1");
        let re = Regex::new(r#":\d+"#).unwrap();
        if !re.is_match(&addr) {
            addr = format!("{}:{}", addr, default_port());
        }

        let addr: SocketAddr = match addr.parse::<SocketAddr>() {
            Err(e) => {
                return Err(D::Error::invalid_value(
                    serde::de::Unexpected::Str(&input.addr),
                    &e.description(),
                ))
            }
            Ok(addr) => addr,
        };

        Ok(Server {
            addr,
            // TODO: Handle error on canonicalize
            root: input.root.canonicalize().unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub enum Origin {
    Any,
    Few(Vec<header::Origin>),
}

impl Default for Origin {
    // Default origin accepts no origins
    fn default() -> Self {
        Origin::Few(Vec::new())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cors {
    pub origin: Origin,
    pub methods: HashSet<Method>,
    pub allowed_headers: HashSet<Ascii<String>>,
    pub exposed_headers: Vec<Ascii<String>>,
    pub allow_credentials: bool,
    pub max_age: Option<u32>,
}

impl<'de> serde::Deserialize<'de> for Origin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum Origin_ {
            Str(String),
            Arr(Vec<String>),
        }

        fn url_to_origin(s: &str) -> Result<header::Origin, url::ParseError> {
            let url: Url = s.parse()?;
            Ok(header::Origin::new(
                url.scheme().to_string(),
                url.host_str().unwrap().to_string(),
                url.port())
            )
        }

        match Origin_::deserialize(deserializer) {
            Ok(Origin_::Str(ref x)) if x == "*" =>
                Ok(Origin::Any),
            Ok(Origin_::Str(x)) =>
                Err(D::Error::invalid_value(
                    serde::de::Unexpected::Str(&x),
                    &"\"*\" or an array of strings",
                )),
            Ok(Origin_::Arr(xs)) => {
                let origins = xs.into_iter()
                    .map(|x| url_to_origin(&x).map_err(|e| D::Error::custom(e.description())))
                    .collect::<Result<Vec<header::Origin>, _>>()?;
                Ok(Origin::Few(origins))
            },
            Err(e) =>
                Err(e)
        }
    }
}

impl<'de> serde::Deserialize<'de> for Cors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize, Debug, Default)]
        struct Cors_ {
            origin: Origin,
            #[serde(default = "default_methods")]
            methods: Vec<String>,
            #[serde(default)]
            allowed_headers: HashSet<String>,
            #[serde(default)]
            exposed_headers: Vec<String>,
            #[serde(default = "default_allowed_credentials")]
            allow_credentials: bool,
            max_age: Option<u32>,
        }

        fn default_methods() -> Vec<String> {
            vec!["GET".to_string(), "HEAD".to_string(), "OPTIONS".to_string()]
        }

        fn default_allowed_credentials() -> bool {
            false
        }

        let input = Cors_::deserialize(deserializer)?;

        let methods = input.methods.into_iter().map(|s| {
            s.parse::<Method>().map_err(|e| D::Error::custom(e.description()))
        }).collect::<Result<Vec<Method>, _>>()?;

        let allowed_headers = HashSet::from_iter(input.allowed_headers.into_iter().map(Ascii::new));
        let exposed_headers = input.exposed_headers.into_iter().map(Ascii::new).collect();

        Ok(Cors {
            origin: input.origin,
            methods: HashSet::from_iter(methods),
            allowed_headers,
            exposed_headers,
            allow_credentials: input.allow_credentials,
            max_age: input.max_age,
        })
    }
}
