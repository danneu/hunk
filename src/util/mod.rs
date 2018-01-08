use std::time::Duration;

const NANOS_PER_MILLI: u64 = 1_000_000;
const MILLIS_PER_SEC: u64 = 1_000;

pub fn as_millis(dur: Duration) -> u64 {
    dur.as_secs() * MILLIS_PER_SEC + (dur.subsec_nanos() as u64) / NANOS_PER_MILLI
}