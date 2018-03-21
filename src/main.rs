extern crate hunk;
extern crate toml;
extern crate unicase;

use std::path::{Path, PathBuf};
use std::env::args;
use std::fs::File;
use std::io::{Read};

use hunk::Config;

fn read_config<P: AsRef<Path>>(path: P) -> Result<Config, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut contents = Vec::new();
    f.read_to_end(&mut contents).map_err(|e| e.to_string())?;
    toml::from_slice(&contents).map_err(|e| e.to_string())
}

fn main() {
    // Parse first argv as path
    let path = args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from("Hunk.toml"));

    let path = match path.canonicalize() {
        Err(e) => {
            eprintln!("could not open config file {:?}: {}", path, e);
            ::std::process::exit(1)
        },
        Ok(path) => path,
    };

    let config = read_config(path.clone())
        .map_err(|e| println!("could not load a config file {:?}. using default settings. {}", path, e))
        .unwrap_or_else(|_| Config::default());

    hunk::serve(config)
}
