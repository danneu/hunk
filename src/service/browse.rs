use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::Path;
use std::time::UNIX_EPOCH;

use futures::{Future, future::ok};
use futures::{stream, Sink, Stream};
use futures_cpupool::CpuPool;
use hyper::{self, header, Chunk, Client, Method, Request, Response, client::HttpConnector,
            server::Service};

use config::{self, Config, Site};
use path;
use mime;
use response;
use service;
use util;

// TODO: Generate ETag, Content-Length

const CSS: &str = include_str!("../assets/browse.css");
const JS: &str = include_str!("../assets/browse.js");

pub struct Browse {
    pub config: &'static Config,
    pub pool: &'static CpuPool,
    // For downstream,
    pub client: &'static Client<HttpConnector>,
    pub remote_ip: IpAddr,
    pub handle: &'static ::tokio_core::reactor::Handle,
}

impl Service for Browse {
    type Request = (&'static Site, Request);
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, (site, req): Self::Request) -> Self::Future {
        trace!("[browse] request {} entered", req.uri());
        let config = self.config;
        let pool = self.pool;
        let client = self.client;
        let remote_ip = self.remote_ip;
        let handle = self.handle;

        let next = move || service::serve::Serve {
            config,
            pool,
            client,
            remote_ip,
            handle,
        };

        // Short-circuit if root or browse opts are not set
        let (root, dotfiles) = match &site.serve {
            None => return next().call((site, req)),
            Some(config::Serve {
                ref root,
                ref dotfiles,
                ..
            }) => (root, dotfiles),
        };

        // Only handle GET, OPTIONS, HEAD
        if *req.method() != Method::Get && *req.method() != Method::Head
            && *req.method() != Method::Options
        {
            return Box::new(next().call((site, req)));
        }

        let entity_path = match path::get_entity_path(&root, req.path()) {
            None => return Box::new(ok(response::not_found())),
            Some(path) => path,
        };

        let future = Box::new(pool.spawn_fn(move || {
            handle_folder(pool, &root, entity_path.as_path(), &dotfiles)
        }));

        Box::new(future.then(move |res| {
            match res {
                // Our handler succeeded, so return its response
                Ok(res) => Box::new(ok(res)),
                // If not a directory or file not found, then continue to next handler
                Err(ref e) if e.raw_os_error() == Some(20) => next().call((site, req)),
                Err(ref e) if e.kind() == io::ErrorKind::NotFound => next().call((site, req)),
                Err(e) => {
                    error!("error in handle_folder: {}", e);
                    Box::new(ok(response::internal_server_error()))
                }
            }
        }))
    }
}

struct FolderItem {
    filename: String,
    href: String,
    metadata: fs::Metadata,
    is_image: bool,
}

fn handle_folder(
    pool: &CpuPool,
    root: &Path,
    path: &Path,
    dotfiles: &bool,
) -> io::Result<Response> {
    let (tx, body) = hyper::Body::pair();

    let mut entries: Vec<FolderItem> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(move |entry| {
            fs::metadata(entry.path()).map(move |metadata| {
                let filename = match entry.path().file_name() {
                    None => return None,
                    Some(filename) => filename.to_string_lossy().to_string(),
                };

                // Skip dotfiles unless we want to serve them
                if !dotfiles && filename.starts_with('.') {
                    return None;
                }

                let is_image = entry
                    .path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| mime::is_image_ext(x))
                    .unwrap_or(false);

                let href = format!(
                    "/{}",
                    entry.path().strip_prefix(&root).unwrap().to_string_lossy()
                );

                Some(FolderItem {
                    filename,
                    href,
                    metadata,
                    is_image,
                })
            })
        })
        .filter_map(Result::ok)
        .filter_map(|item| item)
        .collect();

    // Sort folders first, and the sort by filename a-z
    entries.sort_unstable_by_key(|item| (!item.metadata.is_dir(), item.filename.to_lowercase()));

    let parent_href = path.parent()
        .filter(|parent| parent.starts_with(root))
        .and_then(|parent| parent.strip_prefix(root).ok())
        .and_then(|path| path.to_str())
        .map(|path| format!("/{}", path));

    // e.g. "/", "/foo", "/foo/bar"
    let relative_path = format!(
        "/{}",
        path.strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .to_string()
    );

    let stream = stream::iter_ok(vec![
        Ok(Chunk::from(format!(
            r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>{}</title>
<style>{}</style>
<table><tr><th>Name<th>Size<th>Created
"#,
            relative_path, CSS,
        ))),
        if let Some(parent_href) = parent_href {
            Ok(Chunk::from(format!(
                "<tr><td><a class=\"fo\" href=\"{}\">..<td>—<td>—",
                parent_href
            )))
        } else {
            Ok(Chunk::from(""))
        },
    ]);

    let stream = stream.chain(stream::iter_ok(entries.into_iter().map(|item| {
        //        let class = if item.metadata.is_dir() { "fo" } else { "fi" };

        let class = if item.metadata.is_dir() {
            "fo"
        } else if item.is_image {
            "img"
        } else {
            "fi"
        };

        // string of millis since epoch
        let created = {
            let duration = item.metadata
                .created()
                .map_err(|_| ())
                .and_then(|time| time.duration_since(UNIX_EPOCH).map_err(|_| ()))
                .map(|dur| util::duration_as_millis(dur).to_string())
                .map_err(|()| ());
            duration.unwrap_or_else(|()| "—".to_string())
        };

        let html = format!(
            "\n<tr><td><a href=\"{href}\" class=\"{class}\">{filename}<td>{size}<td class=\"created\">{created}",
            href = item.href,
            filename = item.filename,
            class = class,
            size = if item.metadata.is_file() { item.metadata.len().to_string() } else { "—".to_string() },
            created = created,
        );
        Ok(Chunk::from(html))
    })));

    let js_chunk = Chunk::from(format!("<script>{}</script>", JS));
    let stream = stream.chain(stream::iter_ok(vec![Ok(js_chunk)]));

    let future = tx.send_all(stream);

    pool.spawn(future).forget();

    Ok(Response::new()
        .with_header(header::ContentType::html())
        .with_body(body))
}
