//! A footer page, rendered like the React site's `Pages.tsx`: a header (a
//! back link to the map, the brand), the pages' nav with the current one
//! in gold, then in a 44rem column the title, the intro, Support's
//! showcases, the sections and the "Last updated" line.
//!
//! From search-front/app/src/pages/view.rs (commit 6d370b4). What differs:
//! "Suttas Map" in the title and the brand; five pages (Legal); Support
//! shows the Tipiṭaka app, then the websites (`SiteBlock`, the React
//! `Showcase` with `shape="web"`: the lotus on a tile, landscape
//! screenshots, two in view from `sm`), then `@daily_sutta` — no search
//! showcase; the Instagram mark is the footer's inline SVG; the App Store
//! link's campaign is the React map's `PaliWebMapSupport`. Not here yet:
//! the Acknowledgements' translators table, built from the map's data
//! (`Translators.tsx`), comes with the data (task 006).
//!
//! Inside the shell, above the same `SiteFooter` as the map. A phone: the
//! shell's top bar carries the brand (the header's own is hidden below
//! `sm:`), the nav wraps, a wide table scrolls sideways in its own box.

use leptos::prelude::*;
use leptos_i18n::{td, td_string};
use palicanon_ui_lib::Carousel;

use super::rich::{Span, spans};
use super::{AppBlock, InstagramBlock, PageKind, PageText, Section, SiteBlock, Table, UPDATED};
use crate::catalogue::{Strings, strings};
use crate::components::site_footer::{INSTAGRAM, InstagramIcon};
use crate::head::PageHead;

const H2: &str = "mt-10 font-serif text-xl font-semibold text-heading";
const P: &str = "mt-3 max-w-[66ch] text-[15px] leading-7 text-ink";
const UL: &str = "mt-3 max-w-[66ch] list-disc space-y-1.5 pl-5 text-[15px] leading-7 text-ink";
const A: &str = "text-gold underline underline-offset-4 hover:text-gold-hover";
const TH: &str = "py-2 pr-4 text-left text-xs font-semibold uppercase tracking-wide text-subtle";
const TD: &str = "py-2 pr-4 align-top text-[14px] leading-6 text-ink break-words";
const BTN: &str = "mt-4 inline-flex items-center gap-2 rounded-lg border border-gold/60 px-4 py-2 \
    text-sm font-semibold text-gold transition hover:bg-gold hover:text-on-gold";

/// The App Store link from Support, tagged with the React map's campaign
/// (`appStoreUrl("PaliWebMapSupport")`).
const APP_STORE: &str =
    "https://apps.apple.com/app/apple-store/id6760995639?pt=128678815&ct=PaliWebMapSupport&mt=8";
/// The app's screenshots (554 × 1200), in the order of [`AppBlock::alts`].
const APP_SHOTS: [&str; 5] = [
    "/support/app-main.jpg",
    "/support/app-reader.jpg",
    "/support/app-search.jpg",
    "/support/app-concept-graph.jpg",
    "/support/app-notes.jpg",
];
/// The websites' screenshots with their sizes, in the order of
/// [`PageText::sites`] then of each [`SiteBlock::alts`] (`Pages.tsx`'s
/// `WEB_SHOTS`).
const WEB_SHOTS: [&[(&str, u32, u32)]; 2] = [
    &[
        ("/support/web/palicanon-home.jpg", 2000, 1280),
        ("/support/web/palicanon-results.jpg", 2560, 1600),
    ],
    &[
        ("/support/web/map-whole.jpg", 2560, 1600),
        ("/support/web/map-suttacentral.jpg", 2000, 1280),
        ("/support/web/map-vectorial.jpg", 2000, 1280),
        ("/support/web/map-french.jpg", 2000, 1280),
    ],
];

/// The footer page `kind`, in the app's language.
#[component]
pub(crate) fn FooterPage(kind: PageKind) -> impl IntoView {
    let t = strings();
    move || {
        let locale = t.locale();
        let page = kind.text(locale);
        view! {
            <PageHead
                title=format!("{} — Suttas Map", page.title)
                description=page.description.to_owned()
                path=kind.path().to_owned()
            />
            <div class="mx-auto w-full max-w-[64rem] px-6 pb-20">
                <Header t=t />
                <PagesNav t=t current=kind />
                <main class="mx-auto max-w-[44rem] pt-8">
                    <Body page=page />
                    <p class="mt-10 text-xs text-subtle">
                        {td!(locale, pages_updated, date = UPDATED)}
                    </p>
                </main>
            </div>
        }
    }
}

/// A back link to the map (an arrow, named) and the brand, hidden on a
/// phone where the shell's top bar shows the lotus.
#[component]
fn Header(t: Strings) -> impl IntoView {
    view! {
        <header class="flex items-center gap-3 py-5">
            <a
                href="/"
                aria-label=move || td_string!(t.locale(), panel_back)
                class="grid size-9 place-items-center rounded-full text-muted transition hover:bg-chip hover:text-heading"
            >
                <svg
                    width="18"
                    height="18"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.8"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                >
                    <path d="M13 8H3M7 4 3 8l4 4" />
                </svg>
            </a>
            <a
                href="/"
                class="hidden shrink-0 items-center gap-2 font-serif text-lg font-semibold text-heading select-none sm:flex"
            >
                <img src="/lotus.png" alt="" width="22" height="22" />
                {move || td_string!(t.locale(), brand)}
            </a>
        </header>
    }
}

/// The five pages, the current one gold (and `aria-current`).
#[component]
fn PagesNav(t: Strings, current: PageKind) -> impl IntoView {
    let links = move || {
        PageKind::ALL
            .into_iter()
            .map(|kind| {
                let here = kind == current;
                let class = if here {
                    "text-gold"
                } else {
                    "text-muted transition hover:text-heading"
                };
                view! {
                    <a href=kind.path() aria-current=here.then_some("page") class=class>
                        {kind.label(t.locale())}
                    </a>
                }
            })
            .collect_view()
    };
    view! {
        <nav
            aria-label=move || td_string!(t.locale(), footer_nav)
            class="mx-auto flex max-w-[44rem] flex-wrap gap-x-8 gap-y-2 border-b border-line pt-4 pb-3 text-[15px]"
        >
            {links}
        </nav>
    }
}

/// The title, the intro, Support's showcases, then the sections.
#[component]
fn Body(page: &'static PageText) -> impl IntoView {
    view! {
        <h1 class="font-serif text-3xl font-semibold text-heading">{page.title}</h1>
        {page.intro.iter().map(|p| view! { <p class=P>{rich(p)}</p> }).collect_view()}
        {page.app.as_ref().map(app_block)}
        {page
            .sites
            .iter()
            .zip(WEB_SHOTS)
            .map(|(site, shots)| site_block(site, shots))
            .collect_view()}
        {page.instagram.as_ref().map(instagram_block)}
        {page.sections.iter().map(section).collect_view()}
    }
}

/// Support's showcase of the Tipiṭaka app: icon, name, tagline, the App
/// Store button, the screenshots.
fn app_block(app: &'static AppBlock) -> impl IntoView {
    let slides = APP_SHOTS.iter().zip(app.alts).map(|(src, alt)| {
        view! {
            <img
                src=*src
                alt=*alt
                width="554"
                height="1200"
                loading="lazy"
                class="w-full h-auto rounded-lg"
            />
        }
    });
    view! {
        <section>
            <h2 class=H2>{app.h}</h2>
            <div class="mt-3 flex items-center gap-3">
                <img src="/support/app-icon.jpg" alt="" width="48" height="48" class="rounded-xl" />
                <p class="font-serif text-lg font-semibold text-heading">"Tipiṭaka"</p>
            </div>
            <p class=P>{app.tagline}</p>
            <a href=APP_STORE target="_blank" rel="noopener" class=BTN>
                {format!("{} ↗", app.cta)}
            </a>
            <div class="mt-4">
                <Carousel
                    id="app-shots"
                    label=app.h
                    slide_layout=SHOTS_LAYOUT
                    phone_layout=SHOTS_PHONE_LAYOUT
                >
                    {slides.collect_view()}
                </Carousel>
            </div>
        </section>
    }
}

/// The width of an app screenshot slide from `sm`: four in view (the
/// column's width minus three 0.75 rem gaps, over four).
const SHOTS_LAYOUT: &str = "sm:w-[calc((100%-2.25rem)/4)]";

/// The width of an app screenshot slide below `sm`: two whole slides.
const SHOTS_PHONE_LAYOUT: &str = "w-[calc((100%-0.75rem)/2)]";

/// Support's showcase of one website: the lotus on a tile and the name,
/// the tagline, the button to the site, its landscape screenshots in a
/// carousel with the library's default widths (two in view from `sm`, the
/// next peeking on a phone — the React strip's 380 and 300 px).
fn site_block(
    site: &'static SiteBlock,
    shots: &'static [(&'static str, u32, u32)],
) -> impl IntoView {
    let slides = shots
        .iter()
        .zip(site.alts)
        .map(|((src, width, height), alt)| {
            view! {
                <img
                    src=*src
                    alt=*alt
                    width=*width
                    height=*height
                    loading="lazy"
                    class="w-full h-auto rounded-lg border border-line"
                />
            }
        });
    // This site's own block: its href is the canonical origin.
    let id = if site.href.starts_with(crate::head::CANONICAL_BASE) {
        "map-shots"
    } else {
        "palicanon-shots"
    };
    view! {
        <section>
            <h2 class=H2>{site.h}</h2>
            <div class="mt-3 flex items-center gap-3">
                <span class="grid size-12 place-items-center rounded-xl border border-card-edge bg-card">
                    <img src="/lotus.png" alt="" width="28" height="28" />
                </span>
                <p class="font-serif text-lg font-semibold text-heading">{site.name}</p>
            </div>
            <p class=P>{site.tagline}</p>
            <a href=site.href target="_blank" rel="noopener" class=BTN>
                {format!("{} ↗", site.cta)}
            </a>
            <div class="mt-4">
                <Carousel id=id label=site.h>
                    {slides.collect_view()}
                </Carousel>
            </div>
        </section>
    }
}

/// Support's showcase of `@daily_sutta`.
fn instagram_block(instagram: &'static InstagramBlock) -> impl IntoView {
    view! {
        <section>
            <h2 class=H2>{instagram.h}</h2>
            <div class="mt-3 flex items-center gap-3">
                <InstagramIcon size=28 />
                <p class="font-serif text-lg font-semibold text-heading">"@daily_sutta"</p>
            </div>
            <p class=P>{instagram.tagline}</p>
            <a href=INSTAGRAM target="_blank" rel="noopener" class=BTN>
                {format!("{} ↗", instagram.cta)}
            </a>
        </section>
    }
}

/// One section: its heading, then its table, list, paragraphs, button.
fn section(s: &'static Section) -> impl IntoView {
    view! {
        <section>
            <h2 class=H2>{s.h}</h2>
            {s.table.as_ref().map(table)}
            {(!s.list.is_empty())
                .then(|| {
                    view! {
                        <ul class=UL>
                            {s.list.iter().map(|li| view! { <li>{rich(li)}</li> }).collect_view()}
                        </ul>
                    }
                })}
            {s.p.iter().map(|p| view! { <p class=P>{rich(p)}</p> }).collect_view()}
            {s
                .cta
                .as_ref()
                .map(|cta| {
                    view! {
                        <a href=cta.href target="_blank" rel="noopener" class=BTN>
                            {format!("{} ↗", cta.label)}
                        </a>
                    }
                })}
        </section>
    }
}

/// A section's table, scrolling sideways in its own box on a phone.
fn table(t: &'static Table) -> impl IntoView {
    let rows = t.rows.iter().map(|row| {
        view! { <tr>{row.iter().map(|c| view! { <td class=TD>{rich(c)}</td> }).collect_view()}</tr> }
    });
    view! {
        <div class="mt-3 overflow-x-auto">
            <table class="w-full max-w-[66ch] border-collapse">
                <thead>
                    <tr class="border-b border-line">
                        {t.head.iter().map(|h| view! { <th class=TH>{*h}</th> }).collect_view()}
                    </tr>
                </thead>
                <tbody class="divide-y divide-line">{rows.collect_view()}</tbody>
            </table>
        </div>
    }
}

/// A rich text as markup: bold in the heading colour, a link on this site
/// in the same tab, any other in a new one.
fn rich(text: &'static str) -> impl IntoView {
    spans(text)
        .into_iter()
        .map(|span| match span {
            Span::Text(text) => text.into_any(),
            Span::Bold(text) => view! { <strong class="text-heading">{text}</strong> }.into_any(),
            Span::Link { label, href } if href.starts_with('/') => view! {
                <a href=href class=A>
                    {label}
                </a>
            }
            .into_any(),
            Span::Link { label, href } => view! {
                <a href=href target="_blank" rel="noopener" class=A>
                    {label}
                </a>
            }
            .into_any(),
        })
        .collect_view()
}

/// These tests render markup on the server: they need `ssr`.
#[cfg(all(test, feature = "ssr"))]
mod tests {
    use palicanon_ui_lib::{Language, provide_language};

    use super::*;
    use crate::catalogue::Locale;

    fn render(language: Language, kind: PageKind) -> String {
        Owner::new().with(|| {
            provide_language().set(language);
            leptos_meta::provide_meta_context();
            view! { <FooterPage kind=kind /> }.to_html()
        })
    }

    /// The opening tag of the element holding `needle`.
    fn tag_of<'a>(html: &'a str, needle: &str) -> &'a str {
        let at = html.find(needle).expect(html);
        let start = html[..at].rfind('<').expect(html);
        let end = at + html[at..].find('>').expect(html);
        &html[start..=end]
    }

    /// The pages' nav: the five pages, in the footer's order; the current
    /// one gold and `aria-current`, the others muted — exactly one gold.
    #[test]
    fn the_nav_lists_the_five_pages_with_only_the_current_one_gold() {
        let html = render(Language::En, PageKind::Legal);
        let nav = &html[html.find("<nav").expect(&html)..html.find("</nav>").expect(&html)];
        let mut last = 0;
        for kind in PageKind::ALL {
            let at = nav.find(&format!(r#"href="{}""#, kind.path())).expect(nav);
            assert!(at > last, "in the footer's order: {nav}");
            last = at;
        }
        let current = tag_of(nav, r#"href="/legal""#);
        assert!(current.contains("text-gold"), "{current}");
        assert!(current.contains(r#"aria-current="page""#), "{current}");
        assert_eq!(nav.matches("text-gold").count(), 1, "{nav}");
        assert_eq!(nav.matches("aria-current").count(), 1, "{nav}");
    }

    /// The header: a back link to the map, named, and the brand link.
    #[test]
    fn the_header_has_a_back_link_home_and_the_brand() {
        let html = render(Language::Fr, PageKind::Terms);
        assert!(html.contains(r#"aria-label="Retour""#), "{html}");
        assert!(html.contains(r#"src="/lotus.png""#), "{html}");
        assert!(html.contains("Suttas Map</a>"), "{html}");
        let h1 = html.find("<h1").expect(&html);
        let home_links = html[..h1].matches(r#"href="/""#).count();
        assert_eq!(home_links, 2, "back and brand, before the title: {html}");
    }

    #[test]
    fn the_privacy_page_has_its_table() {
        let html = render(Language::En, PageKind::Privacy);
        assert!(html.contains("<table"), "{html}");
        assert!(html.contains("<thead>"), "{html}");
    }

    /// Support: the app, then the two websites, then Instagram, then the
    /// sections; every screenshot in its carousel with its alt text.
    #[test]
    fn the_support_page_shows_the_app_the_two_sites_then_instagram() {
        let html = render(Language::En, PageKind::Support);
        let order = [
            ">The Tipiṭaka app</h2>",
            r#"id="app-shots""#,
            ">PaliCanon</h2>",
            r#"id="palicanon-shots""#,
            ">Suttas Map</h2>",
            r#"id="map-shots""#,
            ">On Instagram</h2>",
            ">What it costs</h2>",
        ];
        let at: Vec<usize> = order.iter().map(|n| html.find(n).expect(n)).collect();
        assert!(at.windows(2).all(|w| w[0] < w[1]), "{order:?}: {html}");
        assert!(html.contains("ct=PaliWebMapSupport"), "{html}");
        assert!(html.contains(r#"href="https://palicanon.app""#), "{html}");
        assert!(
            html.contains(r#"href="https://map.palicanon.app""#),
            "{html}"
        );
        assert!(
            html.contains(r#"href="https://ko-fi.com/tipitaka""#),
            "{html}"
        );
        assert!(html.contains(&format!(r#"href="{INSTAGRAM}""#)), "{html}");
        assert_eq!(
            html.matches(r#"aria-roledescription="carousel""#).count(),
            3,
            "{html}"
        );
        assert_eq!(html.matches(r#"src="/support/web/"#).count(), 6, "{html}");
        assert!(
            html.contains(r#"alt="SN 56.11 in the vectorial space""#),
            "{html}"
        );
    }

    /// `zip` drops a screenshot without its alt text: every language has
    /// one per screenshot.
    #[test]
    fn every_screenshot_has_its_alt_text_in_every_language() {
        for locale in [Locale::en, Locale::fr, Locale::vi] {
            let page = PageKind::Support.text(locale);
            assert_eq!(page.app.as_ref().expect("app").alts.len(), APP_SHOTS.len());
            assert_eq!(page.sites.len(), WEB_SHOTS.len(), "{locale:?}");
            for (site, shots) in page.sites.iter().zip(WEB_SHOTS) {
                assert_eq!(site.alts.len(), shots.len(), "{locale:?} {}", site.name);
            }
        }
    }

    /// Rich text: bold, an outside link (new tab), a link on this site
    /// (same tab: the Terms' "see Legal").
    #[test]
    fn rich_text_renders_bold_and_both_kinds_of_links() {
        let html = render(Language::En, PageKind::Acknowledgements);
        assert!(
            html.contains(r#"<strong class="text-heading">Digital Pāḷi Dictionary</strong>"#),
            "{html}"
        );
        let outside = tag_of(&html, r#"href="https://suttacentral.net""#);
        assert!(outside.contains(r#"target="_blank""#), "{outside}");
        let html = render(Language::En, PageKind::Terms);
        let inside = tag_of(&html, r#"href="/legal" class="text-gold underline"#);
        assert!(!inside.contains("target"), "{inside}");
    }

    #[test]
    fn every_page_ends_with_its_last_updated_line() {
        // Without the `<!>` hydration markers around the interpolated date.
        let html = render(Language::Fr, PageKind::Legal).replace("<!>", "");
        assert!(html.contains(">Mis à jour le 2026-10-03.</p>"), "{html}");
    }
}
