extern crate hunk;
extern crate toml;
extern crate unicase;

use std::process;
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
        .map(PathBuf::from);

    if let Some(ref path) = path {
       println!("...loading config file from: {:?}", path);
    }

    let path = match path.map(|p| p.canonicalize()) {
        // Path was given and it existed
        Some(Ok(path)) =>
            path,
        // Path given but it was not found
        Some(Err(e)) => {
            eprintln!("...failed to load config file: {}", e);
            process::exit(1);
        },
        // Path not given, so try default config location.
        None => {
            println!("...checking for ./Hunk.toml file");
            PathBuf::from("Hunk.toml")
        }
    };

    let config = read_config(path).expect("...failed to load config file");

    hunk::serve(config)
}
