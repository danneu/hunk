use std::path::Path;
use std::str::FromStr;
use hyper::mime::Mime;
use unicase::Ascii;

mod records;

// PUBLIC

#[derive(Clone)]
pub struct MimeRecord {
    pub mime: Mime,
    pub compressible: bool,
}

pub fn guess_mime_by_path(path: &Path) -> MimeRecord {
    path.extension()
        .and_then(|os| os.to_str())
        .map(|ext| ext_to_mime(ext))
        .unwrap_or_else(|| octet_stream())
}

// PRIVATE

fn octet_stream() -> MimeRecord {
    MimeRecord {
        compressible: false,
        mime: Mime::from_str("application/octet-stream").unwrap(),
    }
}

fn ext_to_mime(ext: &str) -> MimeRecord {
    records::EXT_TO_MIME
        .get(&Ascii::new(ext))
        .map(|m| m.clone())
        .unwrap_or_else(|| octet_stream())
}
