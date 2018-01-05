use std::ops::Range;

use hyper::header::{self, ByteRangeSpec};
use std::cmp;

#[derive(Debug)]
pub enum RequestedRange {
    // Client did not provide a range
    None,

    // Client provided a range but it was invalid
    NotSatisfiable,

    // We can serve the client's requested range
    Satisfiable(Range<u64>),
}

pub fn parse_range_header(header_value: Option<&header::Range>, file_len: u64) -> RequestedRange {
    let max_end = file_len - 1;
    match header_value {
        Some(&header::Range::Bytes(ref byte_ranges)) => {
            if byte_ranges.is_empty() {
                return RequestedRange::NotSatisfiable;
            }
            let range = match byte_ranges.first().unwrap() {
                &ByteRangeSpec::FromTo(start, end) => start..(cmp::min(max_end, end)),
                &ByteRangeSpec::AllFrom(start) => start..max_end,
                &ByteRangeSpec::Last(suffix_len) => {
                    if suffix_len == 0 {
                        return RequestedRange::NotSatisfiable;
                    }
                    (max_end - suffix_len + 1)..max_end
                }
            };

            // VALIDATION

            // Bad range: start > end
            if range.start > range.end {
                return RequestedRange::NotSatisfiable;
            }

            // Bad range: start >= resource length
            // BAD: fileLength=10 and range is "10-"
            // BAD: fileLength=10 and range is "10-10"
            // GOOD: fileLength=10 and range is "9-"
            if range.start > max_end {
                return RequestedRange::NotSatisfiable;
            }

            RequestedRange::Satisfiable(range)
        }
        // We only support byte ranges.
        Some(_) => RequestedRange::NotSatisfiable,
        None => RequestedRange::None,
    }
}
