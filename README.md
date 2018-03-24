# hunk [![Build Status](https://travis-ci.org/danneu/hunk.svg?branch=master)](https://travis-ci.org/danneu/hunk)

A simple, hobby, static-asset / proxy server built with Rust.

![terminal screenshot](/img/splash.png)

## Features

- [x] Lightweight
- [x] Reverse proxy
- [x] Gzip support
- [x] `Range` header / File streaming
- [x] Directory index UI
- [x] Static asset serving
- [x] ETag / Conditional Get / Not Modified

## Install

Download a binary: <https://github.com/danneu/hunk/releases>.

Or compile it yourself with Rust.

    git clone git@github.com:danneu/hunk.git
    cd hunk
    cargo build --release
    
A hunk executable is now available at `./target/release/hunk`.
    
## Usage

Whenever you run `hunk`, hunk will look for a `Hunk.toml` config file in the current directory.

Here's the minimal config:

```toml
[server]
bind = "localhost:3000"
```

Hunk will listen on :3000 but will just respond with 404s 
because no vhosts have been configured.

----

Here's a server that listens on :3000 and serves a directory
of static assets on that port. It also gzips responses, logs
each request to stdout, and renders a directory explorer
for browsing the filesystem:

```toml
[server]
bind = "localhost:3000"

[[vhost]]
host = "localhost:3000"
root = "./public"
[vhost.browse]
[vhost.gzip]
[vhost.log]
```

Notice how `vhost.host` matches `server.bind`.

----

Here's a more advanced config with multiple vhosts.

Requests that have headers "Host: foo.com" or "Host: locahost:4001" are 
proxied to another server listening on port 4001. Those requests 
are also protected by CORS and have whitelisted the catdog.com origin.

Requests that have the header "Host: example.com" are
proxied to another server listening on port 4002. Those requests 
have CORS enabled but have whitelisted all origins.

```toml
[server]
# (Required) The address hunk will listen on
bind = "localhost:3000"

[[vhost]]
# (Required) Can provide one or an array of hosts
host = ["foo.com", "localhost:4001"]
# (Optional) Provide a url if you want to to proxy requests from host -> url
url = "http://localhost:4001"
# (Optional) Static assets directory
root = "foo/public"
[vhost.cors]
origin = ["http://catdog.com"]

[[vhost]]
host = "example.com"
url = "http://localhost:4002"
[vhost.cors]
origin = "*"
```

## Config

### `server` block

```toml
[server]
bind = "localhost:3000"
```

### `vhost` blocks

A config can specify any number of virtual hosts.

```toml
[[vhost]]
host = ""
```

Required:

- `host` (string or array or strings): If an incoming request has a `Host` header that matches one of
    a vhost's hosts, then that vhost block will specify how to handle the request.
    
Optional:

- `url` (url string): Requests to this vhost will be proxied to this `url` where another server will handle it.
- `root` (file path string): Hunk will try to serve the request from this directory of static assets.
- `gzip` (object): Apply the default gzip handler to responses. Hunk will negotiate an encoding.
    - `threshold` (optional int): The minimum byte length for hunk to gzip. Default = 1400.
    
        ```toml
        [[vhost]]
        host = "..."
        [vhost.gzip] # Default gzip middleware
        ```
        
        ```toml
        [[vhost]]
        host = "..."
        [vhost.gzip] 
        threshold = 16000 # Only gzip files larger than 16kb
        ```
- `log` (object): Log request/response to stdout.

    ```toml
    [[vhost]]
    # ...
    [vhost.log]
    ```

- `cors` (object): Apply CORS <https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS> response headers to a vhost.
    - `origin` (array of strings or "*" for wildcard). Ex: `["http://example.com]`.
    - `methods` (optional array of strings). Default: `["GET", "HEAD", "OPTIONS"]`.
    - `allowed_headers` (optional array of strings). Default: `[]`. Ex: `["X-Foo", "X-Bar"]`.
    - `exposed_headers` (optional array of strings). Default: `[]`. Ex: `["X-Exposed"]`.
    - `allow_credentials` (optional bool). Default: `false`. (If true, then origin must not be "*" wildcard)
    - `max_age` (optional int) seconds
    
        ```toml
        [vhost.cors]
        origin = "*"
        methods = ["GET"]
        ```
        
        ```toml
        [[vhost]]
        # Lets foo.com and bar.com send with-credentials cross domain requests
        # thus they will be able to access cookies.
        [vhost.cors]
        origin = ["foo.com", "bar.com"]
        allow_credentials = true
        ```
        
- `browse` (object): When a request hits a folder, render an html page that displays the folder contents
    and lets the user navigate/browse the files.
    
    ![browser screenshot](/img/browse.png)
        
## Development

    git clone https://github.com/danneu/hunk.git
    cd hunk
    cargo install cargo-watch
    CARGO_INCREMENTAL=1 RUST_LOG="hunk" cargo watch -x 'run --bin hunk'
