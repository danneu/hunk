use std::ops::Range;
use std::time;
use std::io;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::fs::File;
use std::sync::Arc;

use futures::{self, Stream, Sink};
use futures_cpupool::CpuPool;
use hyper::{self, Chunk, Header};

use chunks::{ChunkStream};
use base36;
use util;
use mime;

struct ResourceInner {
    inode: u64,
    len: u64,
    mtime: time::SystemTime,
    content_type: mime::MimeRecord,
    file: File,
    pool: CpuPool,
}

#[derive(Clone)]
pub struct Resource {
    inner: Arc<ResourceInner>,
}

impl Resource {
    pub fn new(file: File, pool: CpuPool, content_type: mime::MimeRecord) -> Result<Self, io::Error> {
        let m = file.metadata()?;
        Ok(Resource {
            inner: Arc::new(ResourceInner {
                inode: m.ino(),
                len: m.len(),
                mtime: m.modified()?,
                file,
                pool,
                content_type,
            }),
        })
    }

    pub fn len(&self) -> u64 {
        self.inner.len
    }

    pub fn content_type(&self) -> &mime::MimeRecord {
        &self.inner.content_type
    }

    pub fn last_modified(&self) -> header::HttpDate {
        header::HttpDate::from(self.inner.mtime)
    }

    pub fn etag(&self, strong: bool) -> header::EntityTag {
        let dur = self.inner
            .mtime
            .duration_since(time::UNIX_EPOCH)
            .unwrap_or_else(|_| time::Duration::new(0, 0));

        let tag = format!(
            "{}${}${}",
            base36::encode(self.inner.inode),
            base36::encode(self.len()),
            base36::encode(util::as_millis(dur))
        );

        if strong {
            header::EntityTag::strong(tag)
        } else {
            header::EntityTag::weak(tag)
        }
    }

    pub fn get_range(&self, range: Range<u64>, max_chunk_size: u64) -> ChunkStream {
        let stream =
            futures::stream::unfold((range, Arc::clone(&self.inner)), move |(left, inner)| {
                if left.start == left.end {
                    return None;
                }
                let chunk_size = ::std::cmp::min(max_chunk_size, left.end - left.start) as usize;
                let mut chunk = Vec::with_capacity(chunk_size);
                unsafe { chunk.set_len(chunk_size) };
                let bytes_read = match inner.file.read_at(&mut chunk, left.start) {
                    Err(e) => return Some(Err(hyper::Error::from(e))),
                    Ok(n) => n,
                };
                chunk.truncate(bytes_read);
                Some(Ok((
                    Chunk::from(chunk),
                    (left.start + bytes_read as u64..left.end, inner),
                )))
            });

        let stream: ChunkStream = {
            let (tx, rx) = ::futures::sync::mpsc::channel(0);
            self.inner.pool.spawn(tx.send_all(stream.then(Ok))).forget();
            Box::new(
                rx.map_err(|()| unreachable!())
                    .and_then(::futures::future::result),
            )
        };

        stream
    }
}
