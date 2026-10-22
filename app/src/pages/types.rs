//! The footer pages' shapes, filled by `content_<lang>.rs`: copied from
//! search-front's `app/src/pages/mod.rs` and adapted to the React site's
//! `pages.*` (rendered by its `src/components/Pages.tsx`) — a Legal page,
//! and on Support the websites (`pages.support.web`) instead of
//! search-front's search showcase. Written by `scripts/port_strings.py` in
//! task 005 (removed in task 008, git history); edited by hand since.

/// Every footer page in one language. The "Last updated" line closing
/// each page is the catalogue's `pages_updated`, not here.
#[derive(Debug)]
pub(crate) struct Pages {
    /// Acknowledgements.
    pub acknowledgements: PageText,
    /// Terms of use.
    pub terms: PageText,
    /// Privacy.
    pub privacy: PageText,
    /// Legal notice.
    pub legal: PageText,
    /// Support.
    pub support: PageText,
}

/// One page's text, in the order the page shows it. Paragraphs, list
/// items and table cells are rich text: `**bold**` and `[label](href)`,
/// an `href` starting with `/` being a page of this site.
#[derive(Debug)]
pub(crate) struct PageText {
    /// The `<h1>`, and the document title before " — Suttas Map".
    pub title: &'static str,
    /// The page's meta description.
    pub description: &'static str,
    /// The paragraphs under the title.
    pub intro: &'static [&'static str],
    /// Support only: the Tipiṭaka app's showcase, after the intro.
    pub app: Option<AppBlock>,
    /// Support only: the websites' showcases, after the app's.
    pub sites: &'static [SiteBlock],
    /// Support only: the `@daily_sutta` showcase, after the websites'.
    pub instagram: Option<InstagramBlock>,
    /// The page's sections, in order, after the showcases (on
    /// Acknowledgements, after the translators table).
    pub sections: &'static [Section],
}

/// The Tipiṭaka app's showcase (Support).
#[derive(Debug)]
pub(crate) struct AppBlock {
    /// Its heading.
    pub h: &'static str,
    /// One line on the app.
    pub tagline: &'static str,
    /// The App Store button's label.
    pub cta: &'static str,
    /// The screenshots' alt texts, in the screenshots' order.
    pub alts: &'static [&'static str],
}

/// One website's showcase (Support): PaliCanon, then Suttas Map.
#[derive(Debug)]
pub(crate) struct SiteBlock {
    /// Its heading.
    pub h: &'static str,
    /// The site's name, beside its icon.
    pub name: &'static str,
    /// One line on the site.
    pub tagline: &'static str,
    /// The button's label, linking to the site.
    pub cta: &'static str,
    /// The site's address.
    pub href: &'static str,
    /// The screenshots' alt texts, in the screenshots' order.
    pub alts: &'static [&'static str],
}

/// The `@daily_sutta` showcase (Support).
#[derive(Debug)]
pub(crate) struct InstagramBlock {
    /// Its heading.
    pub h: &'static str,
    /// One line on the account.
    pub tagline: &'static str,
    /// The Instagram button's label.
    pub cta: &'static str,
}

/// One section: a heading, then (in this order, each optional) a table,
/// a list, paragraphs and a button-styled link.
#[derive(Debug)]
pub(crate) struct Section {
    /// The `<h2>`.
    pub h: &'static str,
    /// Paragraphs.
    pub p: &'static [&'static str],
    /// Bullet items.
    pub list: &'static [&'static str],
    /// A table.
    pub table: Option<Table>,
    /// A prominent outside link (Support's Ko-fi).
    pub cta: Option<Cta>,
}

/// A table: its header cells, then its rows.
#[derive(Debug)]
pub(crate) struct Table {
    /// The header cells.
    pub head: &'static [&'static str],
    /// The rows, each a list of cells.
    pub rows: &'static [&'static [&'static str]],
}

/// A button-styled outside link.
#[derive(Debug)]
pub(crate) struct Cta {
    /// What it says.
    pub label: &'static str,
    /// Where it goes.
    pub href: &'static str,
}
