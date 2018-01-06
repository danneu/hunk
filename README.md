# hunk

A simple, single-threaded, hobby static-asset server built with Rust.

My goals are to build a reasonable server while learning and practicing Rust.

## Features

Not exhaustive.

- [x] File streaming
- [x] `Range` support
- [x] Gzip support
- [ ] Directory index UI
- [ ] Tests

## Usage

Download and build hunk:

    git clone git@github.com:danneu/hunk.git
    cd hunk
    cargo build --release
    
A `hunk` executable is now available at `./target/release/hunk`.
    
Serve the "./public" directory:

    ./hunk public -p 3000
    ./hunk public -h 0.0.0.0 -p 80
    
## Config file

Hunk looks for an optional Hunk.toml file in the current directory.

**Note:** Command-line arguments override config file options.

This Hunk.toml file turns on the default gzip handler and
tells the client to cache all files for 4 hours.

    [server]
    host = "0.0.0.0"
    port = 80
    root = "public"
    
    [gzip]
    
    [cache]
    max_age = 14400 
    
If the `[gzip]` and `[cache]` keys did not exist, those features
would simply be turned off.
