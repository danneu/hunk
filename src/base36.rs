// Used for ETags

pub fn encode(mut n: u64) -> String {
    let cap = 1 + if n == 0 {
        // special case since log(36, 0) is -∞
        0
    } else {
        (n as f64).log(BASE as f64).floor() as usize
    };
    let buf = &mut ['0'; MAX_CAP];
    let mut i = MAX_CAP - 1;
    while n >= BASE {
        buf[i] = ALPHABET[(n % BASE) as usize];
        i -= 1;
        n /= BASE
    }
    buf[i] = ALPHABET[n as usize];
    buf[(MAX_CAP - cap)..].into_iter().collect()
}

// PRIVATE

const BASE: u64 = 36;

const MAX_CAP: usize = 13;

const ALPHABET: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];
