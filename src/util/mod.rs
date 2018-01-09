use std::time::Duration;

// Junk drawer of uncategorized functions.

const NANOS_PER_MILLI: u64 = 1_000_000;
const MILLIS_PER_SEC: u64 = 1_000;

pub fn as_millis(dur: Duration) -> u64 {
    dur.as_secs() * MILLIS_PER_SEC + u64::from(dur.subsec_nanos()) / NANOS_PER_MILLI
}
