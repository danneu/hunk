use hyper;
use futures::Stream;

pub type ChunkStream = Box<Stream<Item = hyper::Chunk, Error = hyper::Error> + Send>;
