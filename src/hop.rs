use std::collections::HashSet;

use unicase::Ascii;

// Pass in header.name()
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
