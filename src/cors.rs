use options;
use chunks::ChunkStream;
use hyper::server::{Request, Response};
use hyper::header;
use hyper::Method;
use unicase::Ascii;
use std::collections::HashSet;

lazy_static! {
    static ref SIMPLE_CORS_HEADERS: HashSet<Ascii<String>> = {
        let mut x = HashSet::new();
        x.insert(Ascii::new("Cache-Control".to_string()));
        x.insert(Ascii::new("Content-Language".to_string()));
        x.insert(Ascii::new("Content-Type".to_string()));
        x.insert(Ascii::new("Expires".to_string()));
        x.insert(Ascii::new("Last-Modified".to_string()));
        x.insert(Ascii::new("Pragma".to_string()));
        x
    };
}

// TODO: Incomplete
//
// https://www.w3.org/TR/cors/#resource-processing-model
//
// Returns true if response is finished being handled.
pub fn handle_cors(
    cors: Option<&options::Cors>,
    req: &Request,
    res: &mut Response<ChunkStream>,
) -> bool {
    // Bail if user has no cors options configured
    let cors = match cors {
        None => return false,
        Some(cors) => cors,
    };

    // Bail if request has no Origin header
    let req_origin = match req.headers().get::<header::Origin>() {
        None => return false,
        Some(origin) => origin,
    };

    let allow_origin = match cors.origin {
        options::Origin::Any => true,
        options::Origin::Few(ref alloweds) => alloweds.iter().any(|allowed| allowed == req_origin),
    };

    // Bail if Origin does not match our allowed set
    if !allow_origin {
        return false;
    }

    // Now that valid Origin was given, add Vary header
    res.headers_mut()
        .set(header::Vary::Items(vec![Ascii::new("Origin".to_string())]));

    // Branch the logic between OPTIONS requests and all the rest.
    if *req.method() == Method::Options {
        res.headers_mut().set(header::ContentLength(0));
        res.headers_mut()
            .set(header::ContentType(::hyper::mime::TEXT_PLAIN_UTF_8));

        // Preflight
        let actual_method = match req.headers().get::<header::AccessControlRequestMethod>() {
            // Bail if no method given
            None => return true,
            Some(method) => method,
        };

        // Bail if unapproved method
        if !cors.methods.contains(actual_method) {
            return true;
        }

        let actual_header_keys: Vec<Ascii<String>> = match req.headers()
            .get::<header::AccessControlRequestHeaders>()
            {
                None => Vec::new(),
                Some(&header::AccessControlRequestHeaders(ref keys)) => keys.to_vec(),
            };

        // Bail if any header isn't in our approved set
        if actual_header_keys
            .iter()
            .any(|k| !cors.allowed_headers.contains(k))
            {
                return true;
            }

        // Success, so set the allow origin header.
        res.headers_mut()
            .set(header::AccessControlAllowOrigin::Value(format!(
                "{}",
                req_origin
            )));

        if cors.allow_credentials {
            res.headers_mut().set(header::AccessControlAllowCredentials);
        }

        if let Some(max_age) = cors.max_age {
            res.headers_mut().set(header::AccessControlMaxAge(max_age));
        }

        // Don't have to add these headers if method is a simple cors method.
        res.headers_mut().set(header::AccessControlAllowMethods(
            cors.methods.iter().cloned().collect()
        ));

        // These don't make much sense either since it's just a static file server, but
        // I can always remove it later.
        let nonsimple = |k: &Ascii<String>| -> bool {
            !SIMPLE_CORS_HEADERS.contains(k) || k == &Ascii::new("Content-Type".to_string())
        };
        if actual_header_keys.iter().any(nonsimple) {
            res.headers_mut().set(header::AccessControlAllowHeaders(
                cors.allowed_headers.iter().cloned().collect(),
            ))
        }

        true
    } else {
        // Non-preflight requests
        res.headers_mut()
            .set(header::AccessControlAllowOrigin::Value(format!(
                "{}",
                req_origin
            )));

        if cors.allow_credentials {
            res.headers_mut().set(header::AccessControlAllowCredentials);
        }

        if !cors.exposed_headers.is_empty() {
            res.headers_mut().set(header::AccessControlExposeHeaders(
                cors.exposed_headers.to_vec()
            ))
        }

        false
    }
}
