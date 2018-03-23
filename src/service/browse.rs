use std::fs::File;
use std::path::{Path};
use std::io;
use std::fs::{self, DirEntry};

use futures::{future::ok, Future};
use hyper::{header, Request, Response, Method, server::{Service}};
use maud::{Markup, DOCTYPE, html, PreEscaped};

use config::Browse as Config;
use path;
use response;

const CSS: &str = include_str!("../assets/browse.css");
const JS: &str = include_str!("../assets/browse.js");

#[derive(Debug)]
pub struct Browse<T> {
    config: &'static Option<Config>,
    root: &'static Path,
    next: T,
}

impl<T> Browse<T> {
    pub fn new(config: &'static Option<Config>, root: &'static Path, next: T) -> Self where T: Service + 'static {
        Browse { config, root, next }
    }
}

impl<T> Service for Browse<T> where T: Service<Request = Request, Response = Response> + 'static {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let _config = match *self.config {
            None =>
                return Box::new(self.next.call(req)),
            Some(ref config) =>
                config,
        };

        // Only handle GET, OPTIONS, HEAD
        if *req.method() != Method::Get && *req.method() != Method::Head && *req.method() != Method::Options {
            return Box::new(self.next.call(req))
        }

        let entity_path = match path::get_entity_path(self.root, req.path()) {
            None => return Box::new(ok(response::not_found())),
            Some(path) => path,
        };

        let file = match File::open(&entity_path) {
            Err(_) => return Box::new(ok(response::not_found())),
            Ok(file) => file,
        };

        match file.metadata() {
            Err(_) =>
                Box::new(ok(response::internal_server_error())),
            // Pass files to next ware
            Ok(ref meta) if meta.is_file() =>
                Box::new(self.next.call(req)),
            Ok(ref meta) if meta.is_dir() =>
                match handle_folder(self.root, entity_path.as_path()) {
                    Err(_) =>
                        Box::new(ok(response::internal_server_error())),
                    Ok(response) =>
                        Box::new(ok(response)),
                },
            Ok(_) =>
                Box::new(ok(response::not_found())),
        }
    }
}

struct FolderItem {
    filename: String,
    href: String,
    metadata: fs::Metadata,
}

fn handle_folder(root: &Path, path: &Path) -> io::Result<Response> {
    let entries: Vec<DirEntry> = fs::read_dir(path)?.collect::<io::Result<Vec<DirEntry>>>()?;

    let entries: Vec<Option<FolderItem>> = entries.into_iter()
        .map(|entry| {
            fs::metadata(entry.path()).map(|metadata| {
                let filename = match entry.path().file_name() {
                    None => return None,
                    Some(filename) => filename.to_string_lossy().to_string(),
                };

                // Skip dotfiles
                // if filename.starts_with('.') {
                //     return None
                // }

                let href = format!("/{}", entry.path().strip_prefix(root).unwrap().to_string_lossy());

                Some(FolderItem { filename, href, metadata })
            })
        }).collect::<io::Result<Vec<Option<FolderItem>>>>()?;

    let mut entries: Vec<FolderItem> = entries.into_iter().filter_map(|x| x).collect();

    // Sort folders first, and the sort by filename a-z
    entries.sort_unstable_by_key(|&FolderItem { ref filename, ref metadata, .. }| {
        (!metadata.is_dir(), filename.to_lowercase())
    });

    let parent_href = path.parent()
        .filter(|parent| parent.starts_with(root))
        .and_then(|parent| parent.strip_prefix(root).ok())
        .and_then(|path| path.to_str())
        .map(|path| format!("/{}", path));

    let html = render_html(parent_href, entries).into_string();

    Ok(Response::new()
        .with_header(header::ContentLength(html.len() as u64))
        .with_header(header::ContentType::html())
        .with_body(html))
}

fn render_html(parent_href: Option<String>, entries: Vec<FolderItem>) -> Markup {
    let mut folder_count = 0;
    let mut file_count = 0;

    for item in &entries {
        if item.metadata.is_dir() {
            folder_count += 1
        } else {
            file_count += 1
        }
    }

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8"
                style (PreEscaped(CSS))
            }
            div {
                (folder_count) " directories, "
                (file_count) " files"
            }
            input style="width: 50%" placeholder="Filter" id="filter";
            table style="width: 100%" {
                thead {
                    tr {
                        th ""
                        th ""
                    }
                }

                tbody{
                    @if let Some(href) = parent_href {
                        tr {
                            td a.folder href=(href) { (PreEscaped("&uarr;")) " up" }
                            td ""
                        }
                    }

                    @for (FolderItem { filename, href, metadata }) in entries {
                        tr.entry {
                            td a class=(if metadata.is_dir() { "folder" } else { "file" }) href=(href) {
                                span.filename (filename)
                            }
                            td.size {
                                @if metadata.is_dir() {
                                    "—"
                                } @else {
                                    (pretty_bytes(metadata.len() as f64))
                                }
                            }
                        }
                    }
                }
            }
            script (PreEscaped(JS))
        }
    }
}


fn pretty_bytes(num: f64) -> String {
    use std::cmp;
    const UNITS: &[&str] = &["B", "kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    let negative = if num.is_sign_positive() { "" } else { "-" };
    let num = num.abs();
    if num < 1_f64 {
        return format!("{}{} {}", negative, num, UNITS[0]);
    }
    let delimiter = 1000_f64;
    let exponent = cmp::min((num.ln() / delimiter.ln()).floor() as i32, (UNITS.len() - 1) as i32);
    let pretty_bytes = format!("{:.2}", num / delimiter.powi(exponent)).parse::<f64>().unwrap() * 1_f64;
    let unit = UNITS[exponent as usize];
    format!("{}{} {}", negative, pretty_bytes, unit)
}
