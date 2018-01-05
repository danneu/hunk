use std::path::Path;
use std::str::FromStr;
use hypermime::{self, Mime};

// FIXME: This module is a hack for now.

pub fn is_compressible_path(path: &Path) -> bool {
    path.extension()
        .map(|os| os.to_string_lossy().to_lowercase())
        .map(|ext| match ext.as_ref() {
            "rss" | "js" | "json" | "txt" => true,
            _ => false,
        })
        .unwrap_or(false)
}

pub fn guess_mime_by_path(path: &Path) -> Mime {
    let ext = path.extension().and_then(|os| os.to_str());
    ext.map_or(
        hypermime::APPLICATION_OCTET_STREAM,
        |ext| match ext.to_lowercase().as_str() {
            "rss" => Mime::from_str("application/rss+xml").unwrap(),
            "js" | "mjs" => Mime::from_str("application/javascript").unwrap(),
            "json" | "map" => Mime::from_str("application/json; charset=utf-8").unwrap(),
            "txt" | "text" | "conf" | "def" | "list" | "log" | "in" | "ini" => {
                hypermime::TEXT_PLAIN_UTF_8
            }
            "mp4" => Mime::from_str("video/mp4").unwrap(),
            "webm" => Mime::from_str("video/webm").unwrap(),
            "gif" => hypermime::IMAGE_GIF,
            "jpg" | "jpeg" => hypermime::IMAGE_JPEG,
            "png" => hypermime::IMAGE_PNG,
            _ => hypermime::APPLICATION_OCTET_STREAM,
        },
    )
}
