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
pub fn negotiate_encoding(
    header_value: Option<&header::AcceptEncoding>,
) -> Option<header::Encoding> {
    match header_value {
        None => None,
        Some(&header::AcceptEncoding(ref qitems)) => {
//            let qitems: &Vec<header::QualityItem<header::Encoding>> = qitems;
            let mut qitems = qitems.clone();

            // Sort by client preference descending
            qitems.sort_unstable_by_key(|qi| ::std::cmp::Reverse(qi.quality));

            for qi in qitems {
                match qi.item {
                    header::Encoding::Gzip =>
                        return Some(header::Encoding::Gzip),
                    header::Encoding::Identity =>
                        return None,
                    header::Encoding::EncodingExt(ref ext) if ext == "*" =>
                        // Use gzip if they have no preference
                        return Some(header::Encoding::Gzip),
                    _ =>
                        {}
                }
            }

            None
        }
    }
}
