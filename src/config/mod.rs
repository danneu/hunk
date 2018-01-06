use std::net::Ipv4Addr;

use toml;

pub static DEFAULT_PORT: u16 = 1337;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default)] pub server: Server,
    pub cache: Option<Cache>,
    pub gzip: Option<Gzip>,
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
pub struct Gzip {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Cache {
    pub max_age: u32,
}

pub fn parse(s: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(s)
}
