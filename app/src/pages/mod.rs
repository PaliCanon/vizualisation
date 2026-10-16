//! The footer pages — Acknowledgements, Terms, Privacy, Legal, Support —
//! from the React site (`src/locales/<lang>.json` `pages.*`, rendered like
//! its `src/components/Pages.tsx`).
//!
//! From search-front/app/src/pages/mod.rs (commit 6d370b4). What differs:
//! a Legal page (search-front has none); the shapes live in `types.rs`,
//! written with the texts by `scripts/port_strings.py` (Support carries the
//! websites, `SiteBlock`, instead of search-front's search showcase); the
//! "Last updated" line is the catalogue's `pages_updated`, its date here.
//!
//! The texts are typed Rust data, one module per language
//! (`content_<lang>.rs`), not this service's `leptos_i18n` catalogue: that
//! catalogue reads every JSON array as a plural range, so it cannot hold a
//! page's lists of paragraphs, sections and table rows.

mod content_en;
mod content_fr;
mod content_vi;
mod rich;
mod types;
mod view;

pub(crate) use types::{AppBlock, InstagramBlock, PageText, Pages, Section, SiteBlock, Table};
pub(crate) use view::FooterPage;

use leptos_i18n::td_string;

use crate::catalogue::Locale;

/// The date every page's "Last updated" line gives (`Pages.tsx`'s `UPDATED`).
pub(crate) const UPDATED: &str = "2026-10-03";

/// One footer page, in the order the footer and the pages' own nav list
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PageKind {
    /// Who made the texts the map shows, and their licences.
    Acknowledgements,
    /// The terms of use.
    Terms,
    /// The privacy notice.
    Privacy,
    /// The legal notice: the publisher and the host.
    Legal,
    /// The costs, and how to help.
    Support,
}

impl PageKind {
    /// Every page, in the footer's order.
    pub(crate) const ALL: [Self; 5] = [
        Self::Acknowledgements,
        Self::Terms,
        Self::Privacy,
        Self::Legal,
        Self::Support,
    ];

    /// The page's route.
    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::Acknowledgements => "/acknowledgements",
            Self::Terms => "/terms",
            Self::Privacy => "/privacy",
            Self::Legal => "/legal",
            Self::Support => "/support",
        }
    }

    /// The page's link label in the footer and the pages' nav
    /// (`footer.nav_*`).
    pub(crate) fn label(self, locale: Locale) -> &'static str {
        match self {
            Self::Acknowledgements => td_string!(locale, footer_nav_acknowledgements),
            Self::Terms => td_string!(locale, footer_nav_terms),
            Self::Privacy => td_string!(locale, footer_nav_privacy),
            Self::Legal => td_string!(locale, footer_nav_legal),
            Self::Support => td_string!(locale, footer_nav_support),
        }
    }

    /// The page's text in `locale`.
    pub(crate) fn text(self, locale: Locale) -> &'static PageText {
        let pages = pages(locale);
        match self {
            Self::Acknowledgements => &pages.acknowledgements,
            Self::Terms => &pages.terms,
            Self::Privacy => &pages.privacy,
            Self::Legal => &pages.legal,
            Self::Support => &pages.support,
        }
    }
}

/// Every page in `locale`.
pub(crate) fn pages(locale: Locale) -> &'static Pages {
    match locale {
        Locale::en => &content_en::PAGES,
        Locale::fr => &content_fr::PAGES,
        Locale::vi => &content_vi::PAGES,
    }
}

#[cfg(test)]
mod tests {
    use super::rich::{Span, spans};
    use super::*;
    use crate::route::Route;

    const LOCALES: [Locale; 3] = [Locale::en, Locale::fr, Locale::vi];

    /// Every string of a page, for whole-text checks.
    fn all_text(page: &PageText) -> Vec<&'static str> {
        let mut out = vec![page.title, page.description];
        out.extend(page.intro);
        for s in page.sections {
            out.push(s.h);
            out.extend(s.p);
            out.extend(s.list);
            if let Some(t) = &s.table {
                out.extend(t.head);
                out.extend(t.rows.iter().flat_map(|r| r.iter()));
            }
        }
        out
    }

    /// A link on this site points to a route of the table (the Terms'
    /// "see [Legal](/legal)"), and no placeholder is left unresolved.
    #[test]
    fn every_inside_link_is_a_route_and_no_placeholder_is_left() {
        for locale in LOCALES {
            for kind in PageKind::ALL {
                for text in all_text(kind.text(locale)) {
                    assert!(!text.contains("{{"), "{kind:?} {locale:?}: {text}");
                    for span in spans(text) {
                        if let Span::Link { href, .. } = span
                            && href.starts_with('/')
                        {
                            assert!(Route::parse(href).is_some(), "{kind:?} {locale:?}: {href}");
                        }
                    }
                }
            }
        }
    }

    /// The React site's `{{publisher}}` and `{{contact}}`, resolved.
    #[test]
    fn the_publisher_and_contact_are_resolved() {
        for locale in LOCALES {
            let intro = PageKind::Privacy.text(locale).intro.join(" ");
            assert!(intro.contains("Thibault Rivière"), "{intro}");
            assert!(intro.contains("riviere.thibault@icloud.com"), "{intro}");
        }
    }

    /// Only Support carries the app, websites and Instagram showcases.
    #[test]
    fn only_support_has_the_showcases() {
        for locale in LOCALES {
            for kind in PageKind::ALL {
                let page = kind.text(locale);
                let support = kind == PageKind::Support;
                assert_eq!(page.app.is_some(), support, "{kind:?}");
                assert_eq!(!page.sites.is_empty(), support, "{kind:?}");
                assert_eq!(page.instagram.is_some(), support, "{kind:?}");
            }
        }
    }
}
