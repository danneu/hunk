use std::ops::Range;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::io;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::fs::File;
use std::sync::Arc;
use std::cmp;

use futures_cpupool::CpuPool;
use futures::{stream, Sink, Stream, future::{ok,err}};
use hyper::{self, header, Body, Chunk};

use mime;
use util;
use etag;

struct Inner {
    inode: u64,
    len: u64,
    mtime: SystemTime,
    content_type: mime::MimeRecord,
    file: File,
    pool: CpuPool,
}

#[derive(Clone)]
pub struct Entity {
    inner: Arc<Inner>,
}

#[allow(dead_code)]
pub enum ETagKind {
    Strong,
    Weak,
}

impl Entity {
    pub fn new(
        file: File,
        pool: CpuPool,
        content_type: mime::MimeRecord,
    ) -> Result<Self, io::Error> {
        let m = file.metadata()?;
        Ok(Entity {
            inner: Arc::new(Inner {
                inode: m.ino(),
                len: m.len(),
                mtime: m.modified()?,
                file,
                pool,
                content_type,
            }),
        })
    }

    // This is entity-length, not message-length. e.g. not affected by transfer-encoding.
    pub fn len(&self) -> u64 {
        self.inner.len
    }

    pub fn content_type(&self) -> &mime::MimeRecord {
        &self.inner.content_type
    }

    pub fn last_modified(&self) -> header::HttpDate {
        header::HttpDate::from(self.inner.mtime)
    }

    pub fn etag(&self, kind: &ETagKind) -> header::EntityTag {
        let dur = self.inner
            .mtime
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::new(0, 0));

        let tag = format!(
            "{}${}${}",
            etag::encode(self.inner.inode),
            etag::encode(self.len()),
            etag::encode(util::duration_as_millis(dur))
        );

        match *kind {
            ETagKind::Strong => header::EntityTag::strong(tag),
            ETagKind::Weak => header::EntityTag::weak(tag),
        }
    }

    pub fn get_range(&self, range: Range<u64>, max_chunk_size: u64) -> Body {
        let stream = stream::unfold(
            (range, Arc::clone(&self.inner)),
            move |(remaining, inner)| {
                if remaining.start == remaining.end {
                    return None;
                }

                // Determine size of next chunk
                let chunk_size = cmp::min(max_chunk_size, remaining.end - remaining.start) as usize;

                // Read chunk from file
                let mut chunk = Vec::with_capacity(chunk_size);
                unsafe { chunk.set_len(chunk_size) };
                let bytes_read = match inner.file.read_at(&mut chunk, remaining.start) {
                    Err(e) => return Some(err(hyper::Error::from(e))),
                    Ok(n) => n,
                };
                chunk.truncate(bytes_read);

                Some(ok((
                    Chunk::from(chunk),
                    (remaining.start + bytes_read as u64..remaining.end, inner),
                )))
            },
        );

        let (tx, body) = Body::pair();

        let future = tx.send_all(stream.map(Ok).map_err(|e| {
            // TODO: How should I handle this?
            error!("error while sending get_range stream: {}", e);
            unimplemented!()
        }));

        self.inner.pool.spawn(future).forget();

        body
    }
}
