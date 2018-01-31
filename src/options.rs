use config::Config;
use hyper::{self, header, Method};
use std::str::FromStr;
use std::collections::HashSet;
use unicase::Ascii;
use logger;

// TODO: Consolidate with Config.

// Not sure if this struct makes much sense, yet.
//
// The idea: Options are the result of validating a Config object and
// it represents only the things the handler cares about.
//
// Validation should live here and values should be
// lifted into their final structs (like u32 -> Compression(u32))
// so that the handler doesn't have to do it.

#[derive(Clone)]
pub struct Options {
    pub gzip: Option<Gzip>,
    pub cache: Option<Cache>,
    pub cors: Option<Cors>,
    pub log: Option<Log>,
}

#[derive(Clone)]
pub struct Log {
    pub logger: logger::Logger
}

#[derive(Clone)]
pub struct Gzip {
    pub level: ::flate2::Compression,
    pub threshold: u64,
}

#[derive(Clone)]
pub struct Cache {
    pub max_age: u32,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            gzip: None,
            cache: None,
            cors: None,
            log: None,
        }
    }
}

#[derive(Clone)]
pub enum Origin {
    Any,
    Few(Vec<header::Origin>),
}

#[derive(Clone)]
pub struct Cors {
    pub origin: Origin,
    pub methods: HashSet<hyper::Method>,
    pub allowed_headers: HashSet<Ascii<String>>,
    pub exposed_headers: Vec<Ascii<String>>,
    pub allow_credentials: bool,
    pub max_age: Option<u32>,
}

impl Options {
    pub fn new(config: Config) -> Result<Options, String> {
        let mut o = Options::default();

        if let Some(_) = config.log {
            o.log = Some(Log {
                logger: logger::Logger {
                    dst: logger::Dst::Stdout,
                    format: logger::COMMON_LOG_FORMAT,
                }
            });
        }

        if let Some(opts) = config.gzip {
            if opts.level < 1 || opts.level > 9 {
                return Err(format!("gzip.level must be 1-9. actual={}", opts.level));
            }

            o.gzip = Some(Gzip {
                level: ::flate2::Compression::new(opts.level),
                threshold: opts.threshold,
            })
        };

        if let Some(opts) = config.cache {
            o.cache = Some(Cache {
                max_age: opts.max_age,
            })
        };

        if let Some(cors) = config.cors {
            let mut methods = HashSet::new();
            for method in cors.methods.iter().map(|s| Method::from_str(s).unwrap()) {
                methods.insert(method);
            }

            let mut allowed_headers = HashSet::new();
            for header in cors.allowed_headers.into_iter().map(Ascii::new) {
                allowed_headers.insert(header);
            }

            o.cors = Some(Cors {
                methods,
                allowed_headers,
                exposed_headers: cors.exposed_headers
                    .into_iter()
                    .map(Ascii::new)
                    .collect(),
                allow_credentials: cors.allow_credentials,
                max_age: cors.max_age,
                origin: match cors.origin {
                    None => Origin::Any,
                    Some(urls) => Origin::Few(
                        urls.iter()
                            .map(|s| header::Origin::from_str(s).unwrap())
                            .collect(),
                    ),
                },
            })
        };

        Ok(o)
    }
}
