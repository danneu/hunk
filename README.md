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

