// base92 etag encoding. the goal is to use every valid char according to spec.
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

const BASE: u64 = 92;

// ceil(log base 92 of u64::MAX)
const MAX_CAP: usize = 10;

// entity-tag = [ weak ] opaque-tag
// weak       = "W/"
// opaque-tag = quoted-string
//
// quoted-string  = ( <"> *(qdtext | quoted-pair ) <"> )
// qdtext         = <any TEXT except <">>
//   quoted-pair    = "\" CHAR
//   CHAR           = <any US-ASCII character (octets 0 - 127)>
//   TEXT           = <any OCTET except CTLs, but including LWS>
//   OCTET          = <any 8-bit sequence of data>
//   LWS            = [CRLF] 1*( SP | HT )
//   CTL            = <any US-ASCII control character (octets 0 - 31) and DEL (127)>
//   CRLF           = CR LF
//   CR             = <US-ASCII CR, carriage return (13)>
//   LF             = <US-ASCII LF, linefeed (10)>
//   SP             = <US-ASCII SP, space (32)>
//   HT             = <US-ASCII HT, horizontal-tab (9)>
const ALPHABET: &[char] = &[
    '!', '#', '$', '%', '&', '\'', '(', ')', '*', '+', '-', '.', '/',

    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',

    ':', ';', '<', '=', '>', '?', '@',

    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
    'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',

    '[', '\\', ']', '^', '_', '`',

    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',

    '{', '|', '}', '~',
];


#[test]
fn test_constants() {
    let base = ALPHABET.len() as u64;
    assert_eq!(base, BASE);

    let max_cap = (::std::u64::MAX as f64).log(base as f64).ceil() as usize;
    assert_eq!(max_cap, MAX_CAP);
}
