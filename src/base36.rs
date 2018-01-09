// Encodes backwards, but no point in reversing it
pub fn encode(mut n: u64) -> String {
    let cap = f64::from(36).log(n as f64).ceil() as usize; // max 13
    let mut buf = Vec::with_capacity(cap);
    while n >= 36 {
        buf.push(ALPHABET[(n % 36) as usize]);
        n /= 36
    }
    buf.push(ALPHABET[n as usize]);
    buf.into_iter().collect()
}

// PRIVATE

const ALPHABET: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];
