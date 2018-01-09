use std::net::Ipv4Addr;
use hyper::Method;

use toml;

pub static DEFAULT_PORT: u16 = 1337;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default)] pub server: Server,
    pub cache: Option<Cache>,
    pub gzip: Option<Gzip>,
    pub cors: Option<Cors>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    pub host: Option<Ipv4Addr>,
    pub port: Option<u16>,
    pub root: Option<String>,
}

impl Default for Server {
    fn default() -> Server {
        Server {
            host: Some(Ipv4Addr::localhost()),
            port: Some(DEFAULT_PORT),
            root: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Origin {
    Any,
    Few(Vec<::toml::Value>),
}

impl Default for Origin {
    fn default() -> Origin {
        Origin::Any
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Cors {
    #[serde(default)] pub origin: Option<Vec<String>>,
    #[serde(default = "default_cors_methods")] pub methods: Vec<String>,
    #[serde(default)] pub allowed_headers: Vec<String>,
    #[serde(default = "default_cors_allow_credentials")] pub allow_credentials: bool,
    #[serde(default)] pub max_age: Option<u32>,
    #[serde(default)] pub exposed_headers: Vec<String>,
}

fn default_cors_allow_credentials() -> bool {
    false
}

// TODO: Parse into Methods
fn default_cors_methods() -> Vec<String> {
    vec![
        format!("{}", Method::Get),
        format!("{}", Method::Head),
        format!("{}", Method::Options),
    ]
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Gzip {
    // Must be 1-9
    // IDEA: "best" | "fast" | u32
    #[serde(default = "default_gzip_level")] pub level: u32,
    #[serde(default = "default_gzip_threshold")] pub threshold: u64,
}

// Same as ::flate2::Compression::default()
fn default_gzip_level() -> u32 {
    6
}

fn default_gzip_threshold() -> u64 {
    1400
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Cache {
    pub max_age: u32,
}

pub fn parse(s: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(s)
}
