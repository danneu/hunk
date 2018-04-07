use hyper::mime::{self, Mime};
use std::path::Path;
use unicase::Ascii;
use std::collections::HashSet;

#[macro_use]
mod records;

// PUBLIC

#[derive(Clone)]
pub struct MimeRecord {
    pub mime: Mime,
    pub compressible: bool,
}


pub fn is_image_ext(ext: &str) -> bool {
    lazy_static! {
        static ref IMAGE_EXTS: HashSet<Ascii<&'static str>> = {
            hash_set![
                Ascii::new("jpg"),
                Ascii::new("jpeg"),
                Ascii::new("gif"),
                Ascii::new("bmp"),
                Ascii::new("png"),
            ]
        };
    }

    IMAGE_EXTS.contains(&Ascii::new(ext))
}

pub fn is_mime_compressible(mime: &Mime) -> bool {
    records::COMPRESSIBLE_BY_MIME.contains(mime)
}

pub fn guess_mime_by_path(path: &Path) -> MimeRecord {
    path.extension()
        .and_then(|os| os.to_str())
        .map(ext_to_mime)
        .unwrap_or_else(octet_stream)
}

// PRIVATE

fn octet_stream() -> MimeRecord {
    MimeRecord {
        compressible: false,
        mime: mime::APPLICATION_OCTET_STREAM,
    }
}

fn ext_to_mime(ext: &str) -> MimeRecord {
    records::BY_EXTENSION
        .get(&Ascii::new(ext))
        .cloned()
        .unwrap_or_else(octet_stream)
}
