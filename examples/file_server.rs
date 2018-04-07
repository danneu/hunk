extern crate prox;
extern crate toml;

/// This file server will attempt to serve requests from the ./public folder.
///
/// Prox supports many sites (virtual hosts). But if you only have one,
/// just create a single site entry where site.host == server.bind.

const CONFIG_TOML: &'static str = r#"
    [server]
    bind = "localhost:3000"

    [[site]]
    host = "localhost:3000"
    serve = { root = "examples/public", browse = true }
    log = {}
"#;

fn main() {
    let config = toml::from_str(CONFIG_TOML).unwrap();
    prox::serve(&config)
}