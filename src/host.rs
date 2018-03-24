use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use hyper::header;
use url::Url;

// Note: Case-insensitive
// https://tools.ietf.org/html/rfc3986#section-3.2.2
// https://tools.ietf.org/html/draft-ietf-httpbis-p1-messaging-14#section-9.4
// https://tools.ietf.org/html/draft-ietf-httpbis-p1-messaging-14#section-4.2

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Host {
    pub hostname: String,
    pub port: Option<u16>,
}

impl From<header::Host> for Host {
    fn from(header: header::Host) -> Self {
        Host {
            hostname: header.hostname().to_string(),
            port: header.port(),
        }
    }
}

impl Host {
    pub fn new(hostname: String, port: Option<u16>) -> Self {
        Host { hostname, port }
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn port(&self) -> Option<&u16> {
        self.port.as_ref()
    }

    pub fn to_string(&self) -> String {
        match self.port {
            None => self.hostname().to_string(),
            Some(port) => format!("{}:{}", self.hostname(), port),
        }
    }
}

//impl Hash for Host {
//    fn hash<H: Hasher>(&self, state: &mut H) {
//        self.hostname.hash(state);
//        self.port.hash(state);
//    }
//}

// e.g. localhost:3000 or localhost
impl FromStr for Host {
    type Err = HostParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match format!("http://{}", s)
            .parse::<Url>()
            .map(|url| (url.host_str().map(|s| s.to_string()), url.port()))
        {
            | Ok((Some(hostname), port)) => Ok(Host::new(hostname, port)),
            Ok((None, _)) | Err(_) => Err(HostParseError(())),
        }
    }
}

/// An error returned when parsing a a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostParseError(());

impl fmt::Display for HostParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.description())
    }
}

impl Error for HostParseError {
    fn description(&self) -> &str {
        "host must be \"ipaddress[:port]\" or \"domain[:port]\""
    }
}
