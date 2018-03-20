use std::u64;
use std::time::Duration;

use hyper::{ Body, Response};
use unicase::Ascii;

const NANOS_PER_MILLI: u64 = 1_000_000;
const MILLIS_PER_SEC: u64 = 1_000;

pub fn duration_as_millis(d: Duration) -> u64 {
    d.as_secs() * MILLIS_PER_SEC + u64::from(d.subsec_nanos()) / NANOS_PER_MILLI
}

// If the Vary header is empty, then create it.
// If it's Vary::Any, then do nothing. (i.e. will already Vary)
// If it's Vary::Items, append to the array.
pub fn append_header_vary(res: &mut Response<Body>, item: Ascii<String>) {
    use hyper::header::Vary;

    match res.headers_mut().get_mut::<Vary>() {
        Some(&mut Vary::Any) =>
            return,
        Some(&mut Vary::Items(ref mut xs)) => {
            xs.push(item);
            return
        },
        _ => {}
    }

    res.headers_mut().set(Vary::Items(vec![item]))
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
