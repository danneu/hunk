use std::path::{Path, PathBuf};
use std::str::Utf8Error;

use percent_encoding as pe;

fn percent_decode_str(s: &str) -> Result<String, Utf8Error> {
    pe::percent_decode(s.as_bytes()).decode_utf8().map(|x| x.to_string())
}

// A path is safe if it doesn't try to /./ or /../
fn is_safe_path(path: &Path) -> bool {
    use std::path::Component::{Normal, RootDir};

    path.components().all(|c| match c {
        RootDir | Normal(_) => true,
        _ => false,
    })
}

#[test]
fn test_is_safe_path() {
    assert!(is_safe_path(Path::new("/")));
    assert!(is_safe_path(Path::new("/a/b/c")));
    assert!(!is_safe_path(Path::new("/../a/b/c")));
    assert!(!is_safe_path(Path::new(".")));
    //    assert!(!is_safe_path(Path::new("/a/./c"))); NOTE: . gets dropped here?
}

// Join root with request path to get the asset path candidate.
pub fn get_entity_path(root: &Path, req_path: &str) -> Option<PathBuf> {
    // request path must be absolute
    if !req_path.starts_with('/') {
        return None;
    }

    // Percent-decode the request uri, e.g. GET "/%E4%B8%AD%E6%96%87.txt" should hit "/中文.txt"
    let req_path = percent_decode_str(req_path).ok()?;

    // Security: request path cannot climb directories
    if !is_safe_path(Path::new(&req_path)) {
        return None;
    };

    let mut final_path = root.to_path_buf();
    final_path.push(&req_path[1..]);

    Some(final_path)
}

#[test]
fn test_get_entity_path() {
    assert_eq!(get_entity_path(Path::new("foo"), "/"), Some(PathBuf::from("foo")));
    assert_eq!(get_entity_path(Path::new("foo"), "/bar"), Some(PathBuf::from("foo/bar")));
    assert_eq!(get_entity_path(Path::new("foo"), "/../bar"), None);
    assert_eq!(get_entity_path(Path::new("foo"), "bar"), None);
    assert_eq!(get_entity_path(Path::new("foo"), "/folder/"), Some(PathBuf::from("foo/folder/")));
    assert_eq!(get_entity_path(Path::new("."), "/%E4%B8%AD%E6%96%87.txt"), Some(PathBuf::from("./中文.txt")));
}