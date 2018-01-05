pub mod base36;
mod revvec;

use self::revvec::RevVec;

// PUBLIC

pub struct Codec<'a> {
    encoder: Encoder<'a>,
    decoder: Decoder<'a>,
}

impl<'a> Codec<'a> {
    pub fn new(alphabet: &'a [char]) -> Codec<'a> {
        Codec {
            encoder: Encoder::new(&alphabet),
            decoder: Decoder::new(&alphabet),
        }
    }

    pub fn encode(&self, x: u64) -> String {
        self.encoder.encode(x)
    }

    pub fn decode(&self, x: &str) -> Option<u64> {
        self.decoder.decode(x)
    }
}

// PRIVATE

struct Encoder<'a> {
    alphabet: &'a [char],
}

impl<'a> Encoder<'a> {
    fn new(alphabet: &'a [char]) -> Encoder {
        Encoder { alphabet }
    }

    fn encode(&self, n: u64) -> String {
        let radix = self.alphabet.len();
        let mut n = n;
        let mut digits: RevVec<char> = RevVec::new();
        while n >= radix as u64 {
            let rem = n % (radix as u64);
            digits.push(self.alphabet[rem as usize]);
            n /= radix as u64;
        }
        digits.push(self.alphabet[n as usize]);
        digits.into_iter().collect()
    }
}

struct Decoder<'a> {
    alphabet: &'a [char],
}

impl<'a> Decoder<'a> {
    fn new(alphabet: &'a [char]) -> Decoder {
        Decoder { alphabet }
    }

    fn decode(&self, s: &str) -> Option<u64> {
        let radix = self.alphabet.len();
        let mut idxs: RevVec<usize> = RevVec::with_capacity(s.len());

        for a in s.chars() {
            match self.alphabet.iter().position(|&b: &char| a == b) {
                None =>
                    // Short-circuit
                    return None,
                Some(idx) =>
                    idxs.push(idx)
            }
        }

        Some(idxs.into_iter().enumerate().fold(0, |acc, (power, n)| {
            acc + (n as u64 * ((radix as f64).powi(power as i32) as u64))
        }))
    }
}
