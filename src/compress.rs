use std::io::Write;

use futures_cpupool::CpuPool;
use futures::{Sink, Stream};
use flate2::{Compression, write::GzEncoder};
use hyper::Body;

/// Transforms the body into a new one where all of its chunks will be gzipped.
//
// TODO: Convert back to Stream -> Stream transformation when Hyper 0.12.x releases
// since it'll have Body::wrap_stream().
pub fn gzip(pool: &CpuPool, level: Compression, body: Body) -> Body {
    let stream = body.and_then(move |chunk| {
        let mut encoder = GzEncoder::new(Vec::new(), level);
        encoder
            .write_all(&chunk)
            .and_then(|_| encoder.finish())
            .map(|vec| vec.into())
            .map_err(|e| e.into())
    });

    let (tx, body) = Body::pair();

    pool.spawn(tx.send_all(stream.map(Ok).map_err(|_| unreachable!())))
        .forget();

    body
}
