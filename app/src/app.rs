//! The service's shell: the HTML document ([`shell`], server-rendered), the
//! app root ([`App`], what hydration mounts), and `AppShell`'s rail with the
//! services Search, Learn and Map (this one). From search-front/app/src/app.rs
//! and learn-pali/app/src/app.rs; the routes and pages come in slice S2.

use leptos::prelude::*;
use leptos_i18n::td_string;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;
use palicanon_ui_lib::{
    AppShell, IconName, NavEntry, NavList, NavSection, language_attribute, provide_language,
    theme_attribute,
};

use crate::catalogue::strings;

/// The HTML document around the app, rendered on the server. `data-theme`
/// and `lang` follow the request's cookies ([`theme_attribute`],
/// [`language_attribute`]), so the first paint never flashes the wrong one.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang=language_attribute() data-theme=theme_attribute()>
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="/favicon.ico" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// The app root: what hydration mounts, so it provides the page's one
/// language signal ([`provide_language`]; the server's shell already set
/// `<html lang>` from the very same resolution).
#[component]
pub fn App() -> impl IntoView {
    provide_language();
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/vizualisation.css" />
        <Title text="Suttas Map — PaliCanon" />
        <Router>
            <Layout />
        </Router>
    }
}

/// The rail's entries: the services, Search (the main site), Learn, then Map
/// (this one, the current entry on `/`). `IconName::Grid` stands in for Learn
/// as in learn-pali, `IconName::Motion` for Map, until ui-lib has their icons.
pub(crate) fn nav_entries(locale: crate::catalogue::Locale) -> Vec<NavEntry> {
    vec![
        NavEntry {
            label: td_string!(locale, nav_search),
            icon: IconName::Search,
            href: "https://palicanon.app",
        },
        NavEntry {
            label: td_string!(locale, nav_learn),
            icon: IconName::Grid,
            href: "https://learn.palicanon.app",
        },
        NavEntry {
            label: td_string!(locale, nav_map),
            icon: IconName::Motion,
            href: "/",
        },
    ]
}

/// The rail, following the language and the current path.
fn nav(current: Signal<String>) -> NavList {
    let t = strings();
    let sections = Signal::derive(move || {
        vec![NavSection {
            title: None,
            entries: nav_entries(t.locale()),
        }]
    });
    NavList::new(sections, current)
}

/// The shell around the pages: `AppShell`'s rail, theme and languages.
#[component]
fn Layout() -> impl IntoView {
    let location = use_location();
    view! {
        <AppShell nav=nav(location.pathname.into())>
            <Routes fallback=|| "Not found.">
                <Route path=path!("/") view=|| () />
            </Routes>
        </AppShell>
    }
}

#[cfg(test)]
mod tests {
    use palicanon_ui_lib::Language;

    use super::*;
    use crate::catalogue::Locale;

    #[test]
    fn the_rail_lists_search_learn_then_map() {
        let labels: Vec<&str> = nav_entries(Locale::from(Language::En))
            .iter()
            .map(|entry| entry.label)
            .collect();
        assert_eq!(labels, ["Search", "Learn", "Map"]);
        let hrefs: Vec<&str> = nav_entries(Locale::from(Language::Fr))
            .iter()
            .map(|entry| entry.href)
            .collect();
        assert_eq!(
            hrefs,
            ["https://palicanon.app", "https://learn.palicanon.app", "/"]
        );
    }
}
