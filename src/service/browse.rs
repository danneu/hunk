use std::fs;
use std::io;
use std::net::{IpAddr};
use std::path::Path;

use futures::{Future, future::ok};
use futures::{stream, Sink, Stream};
use futures_cpupool::CpuPool;
use hyper::{self, header, Chunk, Client, Method, Request, Response,
            client::HttpConnector, server::{Service}};

use config::{Config, Site};
use path;
use response;
use service;

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
        let root = match (&site.root, &site.browse) {
            (&Some(ref x), &Some(_)) => x,
            _ => return next().call((site, req)),
        };

        // Only handle GET, OPTIONS, HEAD
        if *req.method() != Method::Get && *req.method() != Method::Head
            && *req.method() != Method::Options
        {
            return Box::new(next().call((site, req)));
        }

        let entity_path = match path::get_entity_path(root, req.path()) {
            None => return Box::new(ok(response::not_found())),
            Some(path) => path,
        };

        let future =
            Box::new(pool.spawn_fn(move || handle_folder(pool, root, entity_path.as_path())));

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
}

const CSS: &str = include_str!("../assets/browse.css");

fn handle_folder(pool: &CpuPool, root: &Path, path: &Path) -> io::Result<Response> {
    let (tx, body) = hyper::Body::pair();

    let mut entries: Vec<FolderItem> = fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(move |entry| {
            fs::metadata(entry.path()).map(move |metadata| {
                let filename = match entry.path().file_name() {
                    None => return None,
                    Some(filename) => filename.to_string_lossy().to_string(),
                };

                let href = format!(
                    "/{}",
                    entry.path().strip_prefix(&root).unwrap().to_string_lossy()
                );

                Some(FolderItem {
                    filename,
                    href,
                    metadata,
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

    let stream = stream::iter_ok(vec![
        Ok(Chunk::from(format!(
            r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<style>{}</style>
<ul>"#,
            CSS
        ))),
        if let Some(parent_href) = parent_href {
            Ok(Chunk::from(format!(
                "<li><a class=\"fo\" href=\"{}\">..",
                parent_href
            )))
        } else {
            Ok(Chunk::from(""))
        },
    ]);

    let stream = stream.chain(stream::iter_ok(entries.into_iter().map(|item| {
        let class = if item.metadata.is_dir() { "fo" } else { "fi" };
        let html = format!(
            "\n<li><a href=\"{href}\" class=\"{class}\">{filename}",
            href = item.href,
            filename = item.filename,
            class = class
        );
        Ok(Chunk::from(html))
    })));

    let future = tx.send_all(stream);

    pool.spawn(future).forget();

    Ok(Response::new()
        .with_header(header::ContentType::html())
        .with_body(body))
}
