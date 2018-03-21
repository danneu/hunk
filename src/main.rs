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
    // Parse first argv as path.
    // If given, then it must exist.
    let path = args()
        .nth(1)
        .map(PathBuf::from)
        .map(|p| p.canonicalize());

    let path = match path {
        // Path was given and it existed
        Some(Ok(path)) =>
            path,
        // Path given but it was not found
        Some(Err(e)) => {
            eprintln!("could not open or find config path");
            ::std::process::exit(1);
        },
        // Path not given, so try default config location.
        None =>
            PathBuf::from("Hunk.toml"),
    };

    let config = read_config(path)
        .unwrap_or_else(|_| Config::default());

    hunk::serve(config)
}
