use std::error::Error;
use std::fmt;
use std::str::FromStr;

use hyper::{Uri, header};

// Note: Case-insensitive
// https://tools.ietf.org/html/rfc3986#section-3.2.2
// https://tools.ietf.org/html/draft-ietf-httpbis-p1-messaging-14#section-9.4
// https://tools.ietf.org/html/draft-ietf-httpbis-p1-messaging-14#section-4.2

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Host {
    hostname: String,
    /// Port defaults to 80
    port: u16,
}

impl From<header::Host> for Host {
    fn from(header: header::Host) -> Self {
        Host {
            hostname: header.hostname().to_string(),
            port: header.port().unwrap_or(80),
        }
    }
}

impl Host {
    pub fn new(hostname: String, port: Option<u16>) -> Self {
        Host { hostname, port: port.unwrap_or(80) }
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.hostname(), self.port())
    }
}

// e.g. localhost:3000 or localhost
impl FromStr for Host {
    type Err = HostParseError;

    // FIXME: Lazy impl
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uri = match format!("{}", s).parse::<Uri>() {
            Ok(uri) => uri,
            Err(_) => return Err(HostParseError(())),
        };

        match uri.host() {
            Some(hostname) => Ok(Host::new(hostname.to_string(), uri.port())),
            None =>  Err(HostParseError(())),
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
