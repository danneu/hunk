# hunk [![Build Status](https://travis-ci.org/danneu/hunk.svg?branch=master)](https://travis-ci.org/danneu/hunk)

A simple, hobby, static-asset server built with Rust.

![terminal screenshot](/img/splash.png)

## Features

Not exhaustive.

- [x] File streaming
- [x] `Range` support
- [x] Gzip support
- [x] Directory index UI
- [x] ETag / Conditional Get / Not Modified
- [x] Middleware composition
- [x] Lightweight (Disk usage: 4.8mb, RAM usage: 850kb idle)

## Usage

You can find compiled binaries on the [releases](https://github.com/danneu/hunk/releases) page.

Or compile them yourself:

    git clone git@github.com:danneu/hunk.git
    cd hunk
    cargo build --release
    
A `hunk` executable is now available at `./target/release/hunk`.

Launch hunk with default settings:

    $ hunk
    [hunk] Listening on localhost:3000
    
## Config file

Hunk looks for an optional Hunk.toml file in the current directory.

```toml
[server]
addr = "localhost:3000"
root = "public"

# Log to req/res to stdout
[log]

# Apply default gzip middleware
[gzip]

# Set cache-control response header
[cache]
max_age = 14400 

# Set default Cross Origin response headers
[cors]

# Show folder browser on folder requests
[browse]
```
    
    
If the `[gzip]` `[cache]`, `[log]`, etc. keys did not exist, those features
would simply be turned off.

None of the top-level entries (meaning the things that look like `[server]`, `[gzip]`, etc.) themselves are required,
but some of them have required fields.

### server

- `addr` (optional string): Ipv4 address + port to bind to. Default = "localhost:3000".
- `root` (optional string): Directory to serve. Default = current directory.

### log

- **(Unimplemented)** `path` (optional string): Destination file for log output. If missing, then logs will be written to stdout.

### gzip

Guesses file types by their file extension and compresses them if they are considered compressible.

For example, .html is compressible but media files like .jpg and .mp4 are not.

### cache

Sets cache-control header for all successful resource responses.

- `max_age` (required int): Duration of **seconds** the client should cache the file for.

### cors

Add [Cross-Origin Resource Sharing](https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS) headers to response.

- `origin` (optional array of strings). If none given, then all origins allowed. Ex: `["http://example.com]`. TODO: Require explicit opt-in with "*" or something.
- `methods` (optional array of strings). Default: `["GET", "HEAD", "OPTIONS"]`.
- `allowed_headers` (optional array of strings). Default: `[]`. Ex: `["X-Foo", "X-Bar"]`.
- `exposed_headers` (optional array of strings). Default: `[]`. Ex: `["X-Exposed"]`.
- `allowed_credentials` (optional bool). Default: `false`.
- `max_age` (optional int) seconds

### browse

Display folder contents browser.

When browse is enabled, a request for a folder will respond with an
HTML page that links to all of the contained files.

![browser screenshot](/img/browse.png)

## Development

    git clone https://github.com/danneu/hunk.git
    cd hunk
    cargo install cargo-watch
    CARGO_INCREMENTAL=1 RUST_LOG=1 cargo watch -x 'run --bin hunk'
