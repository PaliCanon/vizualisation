//! The placeholder-parity check of the app's string catalogue, for
//! `app/build.rs` (task 012): ui-lib's `build/placeholders.rs` when ui-lib
//! sits beside this repository, `None` otherwise (a cargo git checkout, as
//! the PaliCanon app pins this repository), so the build goes on with a
//! warning instead of failing on a missing file. Replaced by ui-lib's
//! `palicanon-i18n-check` crate once ui-lib v0.3.1 is tagged (ui-lib 068).

#[cfg(ui_lib_sibling)]
#[path = "../../../ui-lib/build/placeholders.rs"]
mod placeholders;

/// One line per key whose `{{ name }}` placeholders differ, in any locale,
/// from the first (default) locale's (`locales` are `(code, JSON text)`);
/// `None` when ui-lib's check is not beside this repository.
#[must_use]
pub fn placeholder_mismatches(locales: &[(&str, &str)]) -> Option<Vec<String>> {
    #[cfg(ui_lib_sibling)]
    {
        Some(placeholders::placeholder_mismatches(locales))
    }
    #[cfg(not(ui_lib_sibling))]
    {
        let _ = locales;
        None
    }
}
