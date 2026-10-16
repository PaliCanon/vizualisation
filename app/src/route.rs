//! The typed route table: every path this site answers, as data. A map
//! route is a [`View`] and an optional text uid (`/`, `/mn10`, `/sim`,
//! `/sim/mn10`); a page route is one of the footer pages
//! ([`PageKind`]: `/acknowledgements`, `/terms`, `/privacy`, `/legal`,
//! `/support`); anything else is the 404 ([`Route::parse`] gives `None`).
//!
//! The React site had one map at `/` and its pages (`src/main.tsx`'s
//! `pageFromPath`); design v4 (task 005) gives each view its path and each
//! text its own address. `crate::app`'s `Layout` registers these paths with
//! the router and renders what [`Route::parse`] says, so this table is the
//! one place a path's meaning is decided.

use leptos_i18n::td_string;
use serde::{Deserialize, Serialize};

use crate::catalogue::Locale;
use crate::pages::PageKind;

/// One of the map's views, each at its own path segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum View {
    /// SuttaCentral's parallels, the default view, at `/`.
    Parallels,
    /// The Pāli vectorial space, texts linked above a similarity threshold.
    Sim,
}

impl View {
    /// Every view, in the views menu's order.
    pub(crate) const ALL: [Self; 2] = [Self::Parallels, Self::Sim];

    /// The view's path segment: empty for the default view.
    pub(crate) const fn segment(self) -> &'static str {
        match self {
            Self::Parallels => "",
            Self::Sim => "sim",
        }
    }

    /// Whether the view is the vectorial space's (sim): the view a similarity threshold
    /// (`?th=`) applies to.
    pub(crate) const fn is_vectorial(self) -> bool {
        !matches!(self, Self::Parallels)
    }

    /// The view whose segment is `segment` (never the default view's empty one).
    fn from_segment(segment: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|view| !view.segment().is_empty() && view.segment() == segment)
    }

    /// The view's short name in `locale` (the React views menu's `views.label_*`).
    pub(crate) fn label(self, locale: Locale) -> &'static str {
        match self {
            Self::Parallels => td_string!(locale, views_label_liens),
            Self::Sim => td_string!(locale, views_label_sim),
        }
    }
}

/// A path this site answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    /// The map in `view`, centred on the text `uid` when there is one.
    Map {
        /// The view.
        view: View,
        /// A SuttaCentral uid (`mn10`, `sn56.11`); whether the text exists
        /// is the data's question (task 006), not the path's.
        uid: Option<String>,
    },
    /// A footer page.
    Page(PageKind),
}

impl Route {
    /// The route at `path` (no query, no fragment), `None` for a 404.
    pub(crate) fn parse(path: &str) -> Option<Self> {
        path.strip_prefix('/')?;
        if let Some(kind) = PageKind::ALL.into_iter().find(|kind| kind.path() == path) {
            return Some(Self::Page(kind));
        }
        let (view, uid) = map_address(path)?;
        Some(Self::Map { view, uid })
    }

    /// The route's path, the one [`Route::parse`] reads back.
    pub(crate) fn path(&self) -> String {
        match self {
            Self::Page(kind) => kind.path().to_owned(),
            Self::Map { view, uid } => match (view.segment(), uid) {
                ("", None) => "/".to_owned(),
                ("", Some(uid)) => format!("/{uid}"),
                (segment, None) => format!("/{segment}"),
                (segment, Some(uid)) => format!("/{segment}/{uid}"),
            },
        }
    }
}

/// The map's address in `rest`, the path past where the map is mounted (the map's routes,
/// `crate::fragment::MapRoutes`): the empty path or `/` the parallels, `/<uid>` a text there, `/sim`
/// the vectorial space, `/sim/<uid>` a text there; `None` for anything else (`/MN10`, a
/// third segment, a removed view's `/pca/mn10`): the map's 404. The same rule at the root
/// and under the app's `/map`; a footer page's path is not the map's to decide.
pub(crate) fn map_address(rest: &str) -> Option<(View, Option<String>)> {
    if rest.is_empty() || rest == "/" {
        return Some((View::Parallels, None));
    }
    let mut segments = rest.strip_prefix('/')?.split('/');
    let first = segments.next()?;
    let second = segments.next();
    if segments.next().is_some() {
        return None;
    }
    let (view, uid) = match (View::from_segment(first), second) {
        (Some(view), None) => (view, None),
        (Some(view), Some(uid)) => (view, Some(uid)),
        (None, None) => (View::Parallels, Some(first)),
        (None, Some(_)) => return None,
    };
    if uid.is_some_and(|uid| !is_uid(uid)) {
        return None;
    }
    Some((view, uid.map(str::to_owned)))
}

/// Whether `segment` can be a SuttaCentral uid: not empty, lower-case
/// ASCII letters, digits, `.` and `-` (`mn10`, `sn56.11`,
/// `pli-tv-bu-vb-pj1`). Anything else (`MN10`, `a b`, an encoded
/// character) is a 404 before any data is read.
fn is_uid(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every page, every view without a uid and with one.
    fn every_route() -> Vec<Route> {
        let mut routes: Vec<Route> = PageKind::ALL.into_iter().map(Route::Page).collect();
        for view in View::ALL {
            routes.push(Route::Map { view, uid: None });
            for uid in ["mn10", "sn56.11", "pli-tv-bu-vb-pj1"] {
                routes.push(Route::Map {
                    view,
                    uid: Some(uid.to_owned()),
                });
            }
        }
        routes
    }

    #[test]
    fn route_table_round_trips() {
        for (path, route) in [
            (
                "/",
                Route::Map {
                    view: View::Parallels,
                    uid: None,
                },
            ),
            (
                "/mn10",
                Route::Map {
                    view: View::Parallels,
                    uid: Some("mn10".to_owned()),
                },
            ),
            (
                "/sim/mn10",
                Route::Map {
                    view: View::Sim,
                    uid: Some("mn10".to_owned()),
                },
            ),
            (
                "/sim",
                Route::Map {
                    view: View::Sim,
                    uid: None,
                },
            ),
            ("/legal", Route::Page(PageKind::Legal)),
        ] {
            assert_eq!(Route::parse(path), Some(route.clone()), "{path}");
            assert_eq!(route.path(), path, "{route:?}");
        }
        for route in every_route() {
            let path = route.path();
            assert_eq!(Route::parse(&path), Some(route), "{path}");
        }
    }

    #[test]
    fn an_unknown_path_is_a_404() {
        for path in [
            "/nope/x/y",
            "/sim/a/b",
            "/sim/",
            "/terms/x",
            "//",
            "",
            "nope",
            "/MN10",
            "/mn10?",
            "/pca/mn10",
            "/umap/mn10",
            "/tsne/mn10",
        ] {
            assert_eq!(Route::parse(path), None, "{path}");
        }
    }

    /// The test views removed in 011 (PCA, UMAP, t-SNE) are no view: their
    /// old segment alone reads as a text's uid (a 404 when no text has it),
    /// and with a uid behind it the path is no route.
    #[test]
    fn the_removed_test_views_are_no_view() {
        assert_eq!(View::ALL, [View::Parallels, View::Sim]);
        for segment in ["pca", "umap", "tsne"] {
            assert_eq!(
                Route::parse(&format!("/{segment}")),
                Some(Route::Map {
                    view: View::Parallels,
                    uid: Some(segment.to_owned()),
                }),
                "{segment}"
            );
        }
    }

    /// A page's path is the page, never a text whose uid it would spell.
    #[test]
    fn a_page_path_is_the_page_not_a_uid() {
        for kind in PageKind::ALL {
            assert_eq!(Route::parse(kind.path()), Some(Route::Page(kind)));
        }
    }
}
