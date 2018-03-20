extern crate hunk;
extern crate toml;
extern crate unicase;

use std::fs::File;
use std::io::{self, Read};
use std::error::Error;

use hunk::Config;

fn read_config(path: &str) -> Result<Config, io::Error> {
    let mut f = File::open(path)?;
    let mut contents = Vec::new();
    f.read_to_end(&mut contents)?;
    toml::from_slice(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.description()))
}

fn main() {
    let config = read_config("Hunk.toml").unwrap_or_else(|_| Config::default());

    hunk::serve(config)
}
