use std::time::Duration;
use std::u64;

use unicase::Ascii;

pub fn duration_as_millis(d: Duration) -> u64 {
    d.as_secs() * 1_000 + u64::from(d.subsec_nanos()) / 1_000_000
}

// If the Vary header is empty, then create it.
// If it's Vary::Any, then do nothing. (i.e. will already Vary)
// If it's Vary::Items, append to the array.
pub fn append_header_vary(headers: &mut ::hyper::Headers, item: Ascii<String>) {
    use hyper::header::Vary;

    match headers.get_mut::<Vary>() {
        Some(&mut Vary::Any) => {}
        Some(&mut Vary::Items(ref mut xs)) => xs.push(item),
        None => headers.set(Vary::Items(vec![item])),
    }
}

#[macro_export]
macro_rules! hash_set {
    ( $( $k:expr ),* $(,)? ) => {
        {
            let mut set = ::std::collections::HashSet::new();
            $( set.insert($k); )*
            set
        }
    };
}

#[macro_export]
macro_rules! hash_map {
    ( $( ($k:expr, $v:expr) ),* $(,)? ) => {
        {
            let mut map = ::std::collections::HashMap::new();
            $( map.insert($k, $v); )*
            map
        }
    };
}
