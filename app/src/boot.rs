//! The service's boot configuration, read once from the environment (design
//! v4: the `BOOT` node). Any error stops the service before it listens,
//! naming the variable and never its value (search-front/app/src/boot.rs's
//! rules).
//!
//! - `VIZ_GATE` is required everywhere: `password` (dev) or `off` (prod, or
//!   a local run that says so). `password` without `DEV_PASSWORD` refuses to
//!   start: the gate fails closed.
//! - `ORIGIN_SECRET` as search-front: required in a deployed build (Cloud
//!   Run's `K_SERVICE`, or `LEPTOS_ENV` prod), optional locally.
//!
//! The data files are loaded here too once the `formats` crate lands (task
//! 004): a missing or unparseable file is a boot error like these.

use std::fmt;

use crate::gate::{Gate, Password};
use crate::origin::{Origin, OriginSecret};

/// The variable choosing the password gate: `password` or `off`.
pub const GATE_VAR: &str = "VIZ_GATE";
/// The variable holding the dev password.
pub const PASSWORD_VAR: &str = "DEV_PASSWORD";
/// The variable holding the secret the Worker sends in `x-origin-auth`.
pub const ORIGIN_VAR: &str = "ORIGIN_SECRET";

/// What the service runs with.
#[derive(Debug)]
pub struct Config {
    /// The password gate.
    pub gate: Gate,
    /// The origin guard.
    pub origin: Origin,
}

/// Why the service refuses to start. Names a variable, never a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootError {
    /// `VIZ_GATE` is not set: the gate is never on or off by default.
    GateUnset,
    /// `VIZ_GATE` is neither `password` nor `off`.
    GateUnknown,
    /// `VIZ_GATE=password` without `DEV_PASSWORD`.
    PasswordMissing,
    /// The variable is set to nothing, or to blanks.
    Empty(&'static str),
    /// A deployed build has no origin secret.
    OriginRequired,
}

impl BootError {
    /// The variable the error is about (a name, never a value).
    #[must_use]
    pub fn variable(self) -> &'static str {
        match self {
            BootError::GateUnset | BootError::GateUnknown => GATE_VAR,
            BootError::PasswordMissing => PASSWORD_VAR,
            BootError::Empty(var) => var,
            BootError::OriginRequired => ORIGIN_VAR,
        }
    }
}

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootError::GateUnset => write!(
                f,
                "{GATE_VAR} is not set: say `password` (dev) or `off` (prod, a local run)"
            ),
            BootError::GateUnknown => {
                write!(f, "{GATE_VAR} is neither `password` nor `off`")
            }
            BootError::PasswordMissing => write!(
                f,
                "{GATE_VAR} is `password` but {PASSWORD_VAR} is not set: the gate fails closed"
            ),
            BootError::Empty(var) => write!(f, "{var} is set but empty"),
            BootError::OriginRequired => write!(
                f,
                "{ORIGIN_VAR} is not set: a deployed build answers only the requests that carry it"
            ),
        }
    }
}

impl std::error::Error for BootError {}

/// Whether this is a deployed build: Cloud Run sets `K_SERVICE` in every
/// container, and the image sets `LEPTOS_ENV` to `PROD`.
fn deployed(lookup: &impl Fn(&str) -> Option<String>) -> bool {
    let production = lookup("LEPTOS_ENV").is_some_and(|env| {
        let env = env.trim();
        env.eq_ignore_ascii_case("prod") || env.eq_ignore_ascii_case("production")
    });
    production || lookup("K_SERVICE").is_some()
}

/// The value of `var` through `lookup`, trimmed; blank is an error, not "unset".
fn read(
    lookup: &impl Fn(&str) -> Option<String>,
    var: &'static str,
) -> Result<Option<String>, BootError> {
    match lookup(var) {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Err(BootError::Empty(var)),
        Some(value) => Ok(Some(value.trim().to_owned())),
    }
}

impl Config {
    /// Reads the configuration through `lookup` (`std::env::var(..).ok()` in
    /// the server, a table in tests).
    ///
    /// # Errors
    /// [`BootError`], the first one met, in the order the module doc lists.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Config, BootError> {
        let gate = match read(&lookup, GATE_VAR)?.as_deref() {
            None => return Err(BootError::GateUnset),
            Some("off") => Gate::Off,
            Some("password") => match read(&lookup, PASSWORD_VAR)? {
                Some(password) => Gate::Password(Password::new(password)),
                None => return Err(BootError::PasswordMissing),
            },
            Some(_) => return Err(BootError::GateUnknown),
        };
        let origin = match read(&lookup, ORIGIN_VAR)? {
            Some(secret) => Origin::Checked(OriginSecret::new(secret)),
            None if deployed(&lookup) => return Err(BootError::OriginRequired),
            None => Origin::Open,
        };
        Ok(Config { gate, origin })
    }

    /// The lines logged at boot: which gate and origin check, never a secret.
    #[must_use]
    pub fn log_lines(&self) -> [&'static str; 2] {
        [self.gate.log_line(), self.origin.log_line()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWORD: &str = "gate_canary_boot_7d2a";
    const SECRET: &str = "origin_canary_boot_3e9f";

    /// One boot case: its name, the environment, the error expected.
    type Case = (
        &'static str,
        &'static [(&'static str, &'static str)],
        BootError,
    );

    fn boot(vars: &[(&str, &str)]) -> Result<Config, BootError> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(move |name| {
            vars.iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        })
    }

    #[test]
    fn the_boot_refuses_what_the_design_says_and_names_the_variable() {
        // (case, environment, the error expected)
        let cases: [Case; 7] = [
            ("gate unset", &[], BootError::GateUnset),
            (
                "gate unknown",
                &[(GATE_VAR, "maybe")],
                BootError::GateUnknown,
            ),
            (
                "gate blank",
                &[(GATE_VAR, "  ")],
                BootError::Empty(GATE_VAR),
            ),
            (
                "password without its secret",
                &[(GATE_VAR, "password")],
                BootError::PasswordMissing,
            ),
            (
                "password blank",
                &[(GATE_VAR, "password"), (PASSWORD_VAR, " ")],
                BootError::Empty(PASSWORD_VAR),
            ),
            (
                "deployed without the origin secret (Cloud Run)",
                &[(GATE_VAR, "off"), ("K_SERVICE", "vizualisation-prod")],
                BootError::OriginRequired,
            ),
            (
                "deployed without the origin secret (image)",
                &[(GATE_VAR, "off"), ("LEPTOS_ENV", "PROD")],
                BootError::OriginRequired,
            ),
        ];
        for (case, vars, expected) in cases {
            let error = boot(vars).expect_err(case);
            assert_eq!(error, expected, "{case}");
            assert!(error.to_string().contains(error.variable()), "{case}");
        }
    }

    #[test]
    fn the_boot_accepts_dev_prod_and_a_local_run() {
        let dev = boot(&[
            (GATE_VAR, "password"),
            (PASSWORD_VAR, PASSWORD),
            (ORIGIN_VAR, SECRET),
            ("K_SERVICE", "vizualisation-dev"),
        ])
        .expect("dev");
        assert!(matches!(dev.gate, Gate::Password(_)));
        assert!(matches!(dev.origin, Origin::Checked(_)));
        let prod = boot(&[
            (GATE_VAR, "off"),
            (ORIGIN_VAR, SECRET),
            ("LEPTOS_ENV", "PROD"),
        ])
        .expect("prod");
        assert!(matches!(prod.gate, Gate::Off));
        assert_eq!(prod.log_lines(), ["gate: off", "origin: checked"]);
        let local = boot(&[(GATE_VAR, "off")]).expect("local");
        assert_eq!(local.log_lines(), ["gate: off", "origin: open (local)"]);
    }

    #[test]
    fn no_error_and_no_log_line_shows_a_secret() {
        let dev = boot(&[
            (GATE_VAR, "password"),
            (PASSWORD_VAR, PASSWORD),
            (ORIGIN_VAR, SECRET),
        ])
        .expect("dev");
        let shown = format!("{dev:?} {:?}", dev.log_lines());
        assert!(!shown.contains(PASSWORD) && !shown.contains(SECRET));
        let error = boot(&[(GATE_VAR, "password"), (PASSWORD_VAR, "")]).expect_err("blank");
        assert!(!format!("{error} {error:?}").contains(PASSWORD));
    }
}
