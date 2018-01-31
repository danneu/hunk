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
    
However, you'll need to use a config file to opt into most of Hunk's features.
    
## Config file

Hunk looks for an optional Hunk.toml file in the current directory.

**Note:** Command-line arguments override config file options.

This Hunk.toml file turns on the default gzip handler and
tells the client to cache all files for 4 hours.

    [server]
    host = "0.0.0.0"
    port = 80
    root = "public"
    
    # Log to stdout
    [log]
    
    # Apply gzip to compressible files
    [gzip]
    
    # Set cache-control response header
    [cache]
    max_age = 14400 
    
    # Set Cross Origin response headers
    [cors]
    
    
If the `[gzip]` `[cache]`, or `[log]` keys did not exist, those features
would simply be turned off.

None of the top-level entries (meaning the things that look like `[server]`, `[gzip]`, etc.) themselves are required,
but some of them have required fields.

### server

- `host` (optional string): Ipv4 address to bind to. Default = "127.0.0.1".
- `port` (optional int): Port to bind to. Default = 1337.
- `root` (optional string): Directory to serve. Default = current directory.

### log

- **(Unimplemented)** `path` (optional string): Destination file for log output. If missing, then logs will be written to stdout.

### gzip

Guesses file types by their file extension and compresses them if they are considered compressible.

For example, .html is compressible but media files like .jpg and .mp4 are not.

- `level` (optional int): Set the compression level between 1 (fastest) and 9 (best). Default = 6.
- `threshold` (optional int): Only gzip files when they are at least this long. Default = 1400.
- `also_extensions` (optional array of strings): Also gzip files with these case-insensitive extensions. Default = []. Ex: `also_extensions = ["toml"]`.

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
- `max_age` (optional int)

## Development

    git clone https://github.com/danneu/hunk.git
    cd hunk
    cargo watch -x 'run --bin hunk'
