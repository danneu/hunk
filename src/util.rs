use std::u64;
use std::time::Duration;

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
        Some(&mut Vary::Any) =>
            {},
        Some(&mut Vary::Items(ref mut xs)) =>
            xs.push(item),
        None =>
            headers.set(Vary::Items(vec![item]))
    }
}


// These macros are used for middleware composition.
// I added a case to https://crates.io/crates/pipeline.

#[macro_export]
macro_rules! pipe_fun {
    (&, $ret:expr) => {
        &$ret;
    };
    ((as $typ:ty), $ret:expr) => {
        $ret as $typ;
    };
    ({$fun:expr}, $ret:expr) => {
        $fun($ret);
    };
    ([$fun:ident], $ret:expr) => {
        $ret.$fun();
    };
    (($fun:path[$($arg:expr),*]), $ret:expr) => {
        $fun($($arg,)* $ret);
    };
    (($fun:ident($($arg:expr),*)), $ret:expr) => {
        $fun($ret $(,$arg)*);
    };
    ($fun:ident, $ret:expr) => {
        $fun($ret);
    }
}

#[macro_export]
macro_rules! pipe {
    ( $expr:expr, $($funs:tt),* $(,)? ) => {
        {
            let ret = $expr;
            $(
                let ret = pipe_fun!($funs, ret);
            )*
            ret
        }
    };
}
