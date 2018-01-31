use flate2;
use std::io::Write;
use futures::Stream;

use chunks::ChunkStream;

// Gzip each chunk with the given compression level.
pub fn encode(body: ChunkStream, level: flate2::Compression) -> ChunkStream {
    Box::new(body.and_then(move |chunk| {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), level);
        encoder
            .write(chunk.as_ref())
            .and_then(|_| encoder.finish())
            .map(|vec| vec.into())
            .map_err(|e| e.into())
    }))
}
