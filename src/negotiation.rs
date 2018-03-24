use hyper::header;

// If returns false, then there was an etag match so we should respond with not-modified.
pub fn none_match(header_value: Option<&header::IfNoneMatch>, etag: &header::EntityTag) -> bool {
    match header_value {
        Some(&header::IfNoneMatch::Any) => false,
        Some(&header::IfNoneMatch::Items(ref candidates)) => !(candidates as &[header::EntityTag])
            .iter()
            .any(|candidate| candidate.weak_eq(etag)),
        _ => true,
    }
}

// if it returns false, then we send a precondition-failed response.
pub fn any_match(header_value: Option<&header::IfMatch>, etag: &header::EntityTag) -> bool {
    match header_value {
        None | Some(&header::IfMatch::Any) => true,
        Some(&header::IfMatch::Items(ref candidates)) => (candidates as &[header::EntityTag])
            .iter()
            .any(|candidate| candidate.strong_eq(etag)),
    }
}

// Returns Ok(Encoding) only if it's one of the compressions we support. Else we should not compress.
//
// https://tools.ietf.org/html/rfc7231#section-5.3.4
pub fn negotiate_encoding(
    header_value: Option<&header::AcceptEncoding>,
) -> Option<header::Encoding> {
    let qis = match header_value {
        None => return None,
        Some(&header::AcceptEncoding(ref qis)) => qis,
    };

    let (mut gzip_q, mut identity_q, mut star_q) = (None, None, None);

    for qi in qis {
        match qi.item {
            header::Encoding::Gzip => {
                gzip_q = Some(qi.quality);
            }
            header::Encoding::Identity => {
                identity_q = Some(qi.quality);
            }
            header::Encoding::EncodingExt(ref e) if e == "*" => {
                star_q = Some(qi.quality);
            }
            _ => {}
        };
    }

    let gzip_q = gzip_q.or(star_q).unwrap_or(header::q(0));

    // "If the representation has no content-coding, then it is
    // acceptable by default unless specifically excluded by the
    // Accept-Encoding field stating either "identity;q=0" or "*;q=0"
    // without a more specific entry for "identity"."
    let identity_q = identity_q.or(star_q).unwrap_or(header::q(1));

    if gzip_q > header::q(0) && gzip_q >= identity_q {
        Some(header::Encoding::Gzip)
    } else {
        None
    }
}

#[test]
fn test_negotiate_encoding() {
    use hyper::header::{Header, Raw, AcceptEncoding};
    let parse = |s: &[u8]| AcceptEncoding::parse_header(&Raw::from(&s[..])).unwrap();

    // GZIP
    let ae = parse(b"compress, gzip");
    assert_eq!(negotiate_encoding(Some(&ae)), Some(header::Encoding::Gzip));

    let ae = parse(b"compress;q=0.5, gzip;q=1.0");
    assert_eq!(negotiate_encoding(Some(&ae)), Some(header::Encoding::Gzip));

    let ae = parse(b"gzip;q=1.0, identity; q=0.5, *;q=0");
    assert_eq!(negotiate_encoding(Some(&ae)), Some(header::Encoding::Gzip));

    // NONE
    let ae = parse(b"identity;q=0");
    assert_eq!(negotiate_encoding(Some(&ae)), None);

    let ae = parse(b"*;q=0");
    assert_eq!(negotiate_encoding(Some(&ae)), None);
}
