use std::collections::HashSet;

use unicase::Ascii;

/// Determine if a header is a hop-by-hop header.
///
/// They should be stripped from traffic that passes through the proxy.
pub fn is_hop_header(header: &str) -> bool {
    HOP_HEADERS.contains(&Ascii::new(header))
}

lazy_static! {
    static ref HOP_HEADERS: HashSet<Ascii<&'static str>> = hash_set! [
        Ascii::new("Connection"),
        Ascii::new("Keep-Alive"),
        Ascii::new("Proxy-Authenticate"),
        Ascii::new("Proxy-Authorization"),
        Ascii::new("Te"),
        Ascii::new("Trailers"),
        Ascii::new("Transfer-Encoding"),
        Ascii::new("Upgrade"),
    ];
}

#[test]
fn test_is_hop_header() {
    use hyper::header::Connection;
    let header = Connection::keep_alive();
    assert!(is_hop_header(&format!("{}", header)));
}
