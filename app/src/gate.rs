//! The password gate of the dev environment (design v4: `VIZ_GATE`), the
//! behaviour of the Pages function it replaces (`functions/_middleware.js`):
//! HTTP Basic, any user name, the password compared in constant time; a
//! refusal is a 401 asking for it, never cached. Prod runs the same image
//! with the gate off and no access to the secret (infra).
//!
//! The password lives in [`Password`] (whose `Debug` is redacted) and is in no
//! log line, no response and no error message.

use std::fmt;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use subtle::ConstantTimeEq;

/// The realm the browser shows in its password prompt.
pub const REALM: &str = "PaliCanon viz dev";

/// The dev password. Its `Debug` never shows it.
#[derive(Clone)]
pub struct Password(String);

impl Password {
    /// Wraps the password read from the environment at boot.
    pub fn new(password: impl Into<String>) -> Self {
        Password(password.into())
    }

    /// Whether `candidate` is the password.
    #[must_use]
    pub fn matches(&self, candidate: &[u8]) -> bool {
        let password = self.0.as_bytes();
        password.len() == candidate.len() && password.ct_eq(candidate).into()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}

/// Whether the service asks for the dev password.
#[derive(Clone, Debug)]
pub enum Gate {
    /// No password: prod, or a local run that says so (`VIZ_GATE=off`).
    Off,
    /// Every request must give the password.
    Password(Password),
}

/// The password an `Authorization: Basic …` header gives (what follows the
/// first `:` of the decoded pair), or `None`.
fn given_password(headers: &HeaderMap) -> Option<Vec<u8>> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let (Some(value), None) = (values.next(), values.next()) else {
        return None; // none, or two of them
    };
    let encoded = value.to_str().ok()?.strip_prefix("Basic ")?;
    let pair = STANDARD.decode(encoded.trim()).ok()?;
    let colon = pair.iter().position(|&byte| byte == b':')?;
    Some(pair[colon + 1..].to_vec())
}

impl Gate {
    /// Whether a request with these headers is let through.
    #[must_use]
    pub fn admits(&self, headers: &HeaderMap) -> bool {
        match self {
            Gate::Off => true,
            Gate::Password(password) => {
                given_password(headers).is_some_and(|given| password.matches(&given))
            }
        }
    }

    /// The one line logged at boot: whether the gate is on, never the password.
    #[must_use]
    pub fn log_line(&self) -> &'static str {
        match self {
            Gate::Off => "gate: off",
            Gate::Password(_) => "gate: password",
        }
    }
}

/// The gate, as an axum middleware.
pub async fn guard(State(gate): State<Arc<Gate>>, request: Request, next: Next) -> Response {
    if gate.admits(request.headers()) {
        let mut response = next.run(request).await;
        if matches!(*gate, Gate::Password(_)) {
            // A page behind the password is nobody else's to cache.
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("private, no-store"),
            );
        }
        response
    } else {
        unauthorized()
    }
}

/// The refusal: a 401 asking for the password, nothing cacheable.
fn unauthorized() -> Response {
    let mut response = Response::new(axum::body::Body::from("Password required"));
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    let headers = response.headers_mut();
    let challenge = format!("Basic realm=\"{REALM}\", charset=\"UTF-8\"");
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        headers.insert(header::WWW_AUTHENTICATE, value);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "gate_canary_unit_5c1e";

    fn basic(pair: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let value = format!("Basic {}", STANDARD.encode(pair));
        headers.insert(header::AUTHORIZATION, value.parse().unwrap());
        headers
    }

    #[test]
    fn the_gate_admits_any_user_with_the_password_and_nothing_else() {
        let gate = Gate::Password(Password::new(PASSWORD));
        let longer = format!("{PASSWORD}x");
        // (case, headers, admitted)
        let cases: Vec<(&str, HeaderMap, bool)> = vec![
            ("no header", HeaderMap::new(), false),
            (
                "any user, the password",
                basic(&format!("dev:{PASSWORD}")),
                true,
            ),
            (
                "empty user, the password",
                basic(&format!(":{PASSWORD}")),
                true,
            ),
            (
                "a colon in the password",
                basic(&format!("u:{PASSWORD}:")),
                false,
            ),
            ("wrong password", basic("dev:nope"), false),
            ("longer password", basic(&format!("dev:{longer}")), false),
            ("no colon", basic(PASSWORD), false),
            (
                "bearer, not basic",
                {
                    let mut h = HeaderMap::new();
                    h.insert(
                        header::AUTHORIZATION,
                        format!("Bearer {PASSWORD}").parse().unwrap(),
                    );
                    h
                },
                false,
            ),
            (
                "not base64",
                {
                    let mut h = HeaderMap::new();
                    h.insert(header::AUTHORIZATION, "Basic ***".parse().unwrap());
                    h
                },
                false,
            ),
        ];
        for (case, headers, admitted) in cases {
            assert_eq!(gate.admits(&headers), admitted, "{case}");
        }
    }

    #[test]
    fn an_open_gate_admits_everything() {
        assert!(Gate::Off.admits(&HeaderMap::new()));
    }

    #[test]
    fn the_debug_of_the_password_is_redacted() {
        let gate = Gate::Password(Password::new(PASSWORD));
        assert!(!format!("{gate:?}").contains(PASSWORD));
    }

    #[test]
    fn the_refusal_asks_for_the_password_and_is_not_cached() {
        let response = unauthorized();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(
            response.headers()[header::WWW_AUTHENTICATE]
                .to_str()
                .unwrap()
                .starts_with("Basic realm=\"PaliCanon viz dev\"")
        );
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert!(!format!("{response:?}").contains(PASSWORD));
    }
}
