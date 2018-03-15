use hyper;
use futures::stream::Stream;

pub type ChunkStream = Box<Stream<Item = hyper::Chunk, Error = hyper::Error> + Send>;
