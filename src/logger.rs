use chrono::prelude::Utc;

use hyper::{Request, Response};

use chunks::ChunkStream;


// FIXME: Very lazy implementation.


#[derive(Clone)]
pub enum Dst {
    Stdout
}

#[derive(Clone)]
pub struct Logger {
    pub format: &'static str,
    pub dst: Dst
}

impl Logger {
    pub fn log(&self, req: &Request, res: &Response<ChunkStream>) {
        let now = Utc::now();
        let remote_port = req.remote_addr().unwrap().port();
        let remote_host = req.remote_addr().unwrap().ip();
        let method = format!("{}", req.method());
        let path = req.uri().path();
        let query = req.uri().query().unwrap_or_else(|| "");
        let url = if query.is_empty() {
            format!("{}", path)
        } else {
            format!("{}?{}", path, query)
        };
        let proto = format!("{}", req.version());
        let status = format!("{}", res.status().as_u16());
        // TODO: Count bytes sent
        let bytes_tx = format!("{}", -1);

        let line = self.format
            .replace(":remote_host", &format!("{}", remote_host))
            .replace(":remote_port", &format!("{}", remote_port))
            .replace(":date_clf", &format!("{}",now.format(date_formats::CLF)))
            .replace(":date_iso8601", &format!("{}",now.format(date_formats::ISO_8601_UTC)))
            .replace(":method", &method)
            .replace(":path", path)
            .replace(":url", &url)
            .replace(":proto", &proto)
            .replace(":status", &status)
            .replace(":bytes_tx", &bytes_tx);

        match self.dst {
            Dst::Stdout => println!("{}", line)
        }
    }
}

pub static COMMON_LOG_FORMAT: &'static str = ":remote_host - - [:date_clf] \":method :url :proto\" :status :bytes_tx";

#[allow(dead_code)]
mod date_formats {
    pub static CLF: &'static str = "%d/%b/%Y:%H:%M:%S %z";
    // ISO-8601, e.g. javascript's new Date().toISOString()
    pub static ISO_8601_UTC: &'static str = "%Y-%m-%dT%H:%M:%S%.3fZ";
    // When offset from UTC != 0, then the offset is displayed instead of "Z".
    pub static ISO_8601_OFFSET: &'static str = "%Y-%m-%dT%H:%M:%S%.3f%:z";
}
