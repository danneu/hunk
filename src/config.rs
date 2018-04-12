use std::collections::HashSet;
use std::error::Error;
use std::iter::FromIterator;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use hyper::{header, Method};
use serde;
use unicase::Ascii;
use url::{self, Url};

use host::Host;

/// Configures the proxy server.
#[derive(Deserialize, Debug, Clone, Default)]
pub struct Config {
    /// Top-level prox server config.
    pub server: Server,

    /// The list of virtual host configurations to handle.
    #[serde(rename = "site")]
    #[serde(default)]
    pub sites: Vec<Site>,
}

/// Configures top-level concerns like which port to bind to.
#[derive(Debug, Clone)]
pub struct Server {
    /// Bind the prox server to this address.
    pub bind: SocketAddr,
    pub timeouts: Timeouts,
}

#[derive(Debug, Clone)]
pub struct Timeouts {
    /// The amount of time to wait for a site to start responding.
    pub connect: Duration,
}

impl Default for Timeouts {
    fn default() -> Timeouts {
        Timeouts {
            connect: Duration::from_secs(5),
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Server {
            bind: default_bind().parse().unwrap(),
            timeouts: Timeouts::default(),
        }
    }
}

fn default_bind() -> String {
    format!("127.0.0.1:{}", default_port())
}

fn default_port() -> u32 {
    3000
}

/// A Site tells prox how to handle requests that match the host.
#[derive(Debug, Clone, Default)]
pub struct Site {
    /// The value of the request Host header that will map to this site.
    pub host: Vec<Host>,

    /// Proxy requests to this url. Example: `http://localhost:3001`.
    pub url: Option<Url>,

    /// Configure static-file serving.
    pub serve: Option<Serve>,

    /// Configure response gzipping.
    pub gzip: Option<Gzip>,

    /// Configure request/response logging.
    pub log: Option<Log>,

    /// Configure CORS.
    pub cors: Option<Cors>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Serve {
    /// The filesystem path to the folder to serve.
    ///
    /// May be relative or absolute.
    ///
    /// On each request, the request path will be looked up in the root folder.
    /// If it exists, then the file is served. Else, the request is passed down the chain.
    pub root: PathBuf,

    /// Show the folder browser UI. Default: `false`.
    #[serde(default)]
    pub browse: bool,

    /// Show and serve files that start with dot ("."). Default: `false`.
    #[serde(default)]
    pub dotfiles: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Gzip {
    // Gzip: 0-9
    // We default 1 because it has the maximum compression to cpu ratio.
    // pub level: _,
    /// The minimum file size that will be gzipped. Default: `1400`.
    #[serde(default = "default_gzip_threshold")]
    pub threshold: u64,
}

fn default_gzip_threshold() -> u64 {
    1400
}

#[derive(Deserialize, Debug, Clone)]
pub struct Log {
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_format() -> String {
    super::service::log::COMMON_LOG_FORMAT.to_string()
}

impl<'de> serde::Deserialize<'de> for Site {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize, Debug, Clone)]
        #[serde(untagged)]
        enum Hosts_ {
            Str(Host),
            Arr(Vec<Host>),
        }

        #[derive(Deserialize, Debug, Clone)]
        struct Site_ {
            host: Hosts_,
            url: Option<String>,
            serve: Option<Serve>,
            gzip: Option<Gzip>,
            log: Option<Log>,
            cors: Option<Cors>,
        }

        let mut input: Site_ = Site_::deserialize(deserializer)?;

        // FIXME: This file is a mess.
        let url = match input.clone().url.map(|url| url.parse::<Url>()) {
            None => None,
            Some(Ok(ref x)) => Some(x.clone()),
            Some(Err(e)) => {
                return Err(D::Error::invalid_value(
                    serde::de::Unexpected::Str(&input.clone().url.unwrap()),
                    &e.description(),
                ))
            }
        };

        let host = match input.host {
            Hosts_::Str(x) => vec![x],
            Hosts_::Arr(xs) => xs,
        };

        // Canonicalize the root just so it's more helpful to see in the boot message.
        // We don't care if canonicalize fails because we check the folder every request so
        // even if the folder doesn't exist now, it can exist in the future.
        if let Some(ref serve) = input.serve {
            if let Ok(root) = serve.root.canonicalize() {
                input.serve = Some(Serve { root, ..serve.clone() })
            }
        }

        Ok(Site {
            host,
            url,
            serve: input.serve,
            gzip: input.gzip,
            log: input.log,
            cors: input.cors,
        })
    }
}

impl<'de> serde::Deserialize<'de> for Host {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let input: String = String::deserialize(deserializer)?;

        match input.parse::<Host>() {
            Err(e) => Err(D::Error::invalid_value(
                serde::de::Unexpected::Str(&input),
                &e.description(),
            )),
            Ok(host) => Ok(host),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize)]
        struct Http_ {
            #[serde(default = "default_bind")]
            bind: String,
            timeouts: Option<Timeouts>,
        }

        let input = Http_::deserialize(deserializer)?;

        // Allow localhost short-hand
        let bind = input.bind.replace("localhost", "127.0.0.1");

        let bind: SocketAddr = match bind.parse::<SocketAddr>() {
            Err(e) => {
                return Err(D::Error::invalid_value(
                    serde::de::Unexpected::Str(&input.bind),
                    &e.description(),
                ))
            }
            Ok(bind) => bind,
        };

        Ok(Server {
            bind,
            timeouts: input.timeouts.unwrap_or_else(Timeouts::default),
        })
    }
}

// CORS

#[derive(Debug, Clone)]
pub enum CorsOrigin {
    Any,
    Few(Vec<header::Origin>),
}

impl Default for CorsOrigin {
    // Default origin accepts no origins
    fn default() -> Self {
        CorsOrigin::Few(Vec::new())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Cors {
    pub origin: CorsOrigin,
    pub methods: HashSet<Method>,
    pub allowed_headers: HashSet<Ascii<String>>,
    pub exposed_headers: Vec<Ascii<String>>,
    pub allow_credentials: bool,
    pub max_age: Option<u32>,
}

impl<'de> serde::Deserialize<'de> for CorsOrigin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        enum CorsOrigin_ {
            Str(String),
            Arr(Vec<String>),
        }

        fn url_to_origin(s: &str) -> Result<header::Origin, url::ParseError> {
            let url: Url = s.parse()?;
            Ok(header::Origin::new(
                url.scheme().to_string(),
                url.host_str().unwrap().to_string(),
                url.port(),
            ))
        }

        match CorsOrigin_::deserialize(deserializer) {
            Ok(CorsOrigin_::Str(ref x)) if x == "*" => Ok(CorsOrigin::Any),
            Ok(CorsOrigin_::Str(x)) => Err(D::Error::invalid_value(
                serde::de::Unexpected::Str(&x),
                &"\"*\" or an array of strings",
            )),
            Ok(CorsOrigin_::Arr(xs)) => {
                let origins = xs.into_iter()
                    .map(|x| url_to_origin(&x).map_err(|e| D::Error::custom(e.description())))
                    .collect::<Result<Vec<header::Origin>, _>>()?;
                Ok(CorsOrigin::Few(origins))
            }
            Err(e) => Err(e),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Cors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize, Debug, Default)]
        struct Cors_ {
            origin: CorsOrigin,
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

        let methods = input
            .methods
            .into_iter()
            .map(|s| {
                s.parse::<Method>()
                    .map_err(|e| D::Error::custom(e.description()))
            })
            .collect::<Result<Vec<Method>, _>>()?;

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

impl<'de> serde::Deserialize<'de> for Timeouts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug, Default)]
        struct Timeouts_ {
            connect: u64,
        }

        let input = Timeouts_::deserialize(deserializer)?;

        let connect = Duration::from_millis(input.connect);

        Ok(Timeouts { connect })
    }
}
