//! The origin guard (copied from search-front/app/src/origin.rs, its task
//! 007): the service answers Cloudflare's Worker only.
//!
//! The Cloud Run URL is public, so a request that skipped Cloudflare (its
//! WAF, its per-IP limit) could still spend the API token. The Worker adds a
//! secret in `x-origin-auth` to every request it relays; this layer, the
//! outermost of the router, refuses any request that does not carry it with
//! a 403 and an empty body. The secret is compared in constant time, lives in
//! [`OriginSecret`] (whose `Debug` is redacted) and is in no log line, no
//! response and no error message.

use std::fmt;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use subtle::ConstantTimeEq;

/// The header the Worker puts the secret in.
pub const ORIGIN_HEADER: &str = "x-origin-auth";

/// The Worker's secret. Its `Debug` never shows it.
#[derive(Clone)]
pub struct OriginSecret(String);

impl OriginSecret {
    /// Wraps the secret read from the environment at boot.
    pub fn new(secret: impl Into<String>) -> Self {
        OriginSecret(secret.into())
    }

    /// Whether `candidate` is the secret.
    #[must_use]
    pub fn matches(&self, candidate: &[u8]) -> bool {
        let secret = self.0.as_bytes();
        // Only the length is public: a different length is refused without
        // comparing; equal lengths are compared without an early exit.
        secret.len() == candidate.len() && secret.ct_eq(candidate).into()
    }
}

impl fmt::Debug for OriginSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OriginSecret(<redacted>)")
    }
}

/// Whether the service checks where a request came from.
#[derive(Clone, Debug)]
pub enum Origin {
    /// No check: a local run.
    Open,
    /// Every request must carry the secret.
    Checked(OriginSecret),
}

impl Origin {
    /// Whether a request with these headers is let through.
    #[must_use]
    pub fn admits(&self, headers: &HeaderMap) -> bool {
        match self {
            Origin::Open => true,
            Origin::Checked(secret) => {
                let mut values = headers.get_all(ORIGIN_HEADER).iter();
                match (values.next(), values.next()) {
                    (Some(value), None) => secret.matches(value.as_bytes()),
                    // No header, or two of them: refused.
                    _ => false,
                }
            }
        }
    }

    /// The one line logged at boot: whether the check is on, never the secret.
    #[must_use]
    pub fn log_line(&self) -> &'static str {
        match self {
            Origin::Open => "origin: open (local)",
            Origin::Checked(_) => "origin: checked",
        }
    }
}

/// The guard, as an axum middleware.
pub async fn guard(State(origin): State<Arc<Origin>>, request: Request, next: Next) -> Response {
    if origin.admits(request.headers()) {
        next.run(request).await
    } else {
        forbidden()
    }
}

/// The refusal: a 403, nothing cacheable, nothing in the body.
fn forbidden() -> Response {
    let mut response = Response::new(axum::body::Body::empty());
    *response.status_mut() = StatusCode::FORBIDDEN;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "origin_canary_unit_41aa";

    #[test]
    fn only_the_exact_secret_matches() {
        let secret = OriginSecret::new(SECRET);
        assert!(secret.matches(SECRET.as_bytes()));
        let longer = format!("{SECRET}x");
        let same_length = SECRET.replace('a', "b");
        assert_eq!(same_length.len(), SECRET.len());
        for (case, candidate) in [
            ("empty", ""),
            ("prefix", &SECRET[..SECRET.len() - 1]),
            ("suffix", &SECRET[1..]),
            ("longer", longer.as_str()),
            ("same length, other bytes", same_length.as_str()),
            ("spaces", "                       "),
        ] {
            assert!(!secret.matches(candidate.as_bytes()), "{case}");
        }
    }

    #[test]
    fn the_debug_of_a_secret_is_redacted() {
        let secret = OriginSecret::new(SECRET);
        assert!(!format!("{secret:?}").contains(SECRET));
        let checked = Origin::Checked(secret);
        assert!(!format!("{checked:?}").contains(SECRET));
    }

    #[test]
    fn an_open_origin_admits_everything_a_checked_one_wants_one_right_header() {
        let mut headers = HeaderMap::new();
        assert!(Origin::Open.admits(&headers));
        let checked = Origin::Checked(OriginSecret::new(SECRET));
        assert!(!checked.admits(&headers), "no header");
        headers.insert(ORIGIN_HEADER, SECRET.parse().unwrap());
        assert!(checked.admits(&headers), "the right header");
        headers.append(ORIGIN_HEADER, SECRET.parse().unwrap());
        assert!(!checked.admits(&headers), "twice");
    }

    #[test]
    fn a_header_that_is_not_utf8_text_is_refused() {
        let mut headers = HeaderMap::new();
        headers.insert(
            ORIGIN_HEADER,
            header::HeaderValue::from_bytes(&[0xff, 0xfe, 0x80]).unwrap(),
        );
        let checked = Origin::Checked(OriginSecret::new(SECRET));
        assert!(!checked.admits(&headers));
    }
}
