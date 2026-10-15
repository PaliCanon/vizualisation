//! The page footer: ui-lib's [`Footer`] with the site's five pages
//! (Acknowledgements · Terms · Privacy · Legal · Support, each a page of
//! `crate::pages`), as the React `Footer.tsx`. At its left, this build's
//! env badge (`crate::app`'s [`BADGE`]) and the three outbound icons
//! (App Store, Instagram, Ko-fi: the React `SocialLinks`).
//!
//! From learn-pali/app/src/components/site_footer.rs (commit cd65d7b; the
//! shell's `footer` slot owns the wrapper that pushes it to the bottom of
//! a short page, this component adds none) and search-front's (the links
//! are this site's own pages). What differs: five links, Legal included;
//! the icons, which ui-lib's `Footer` has no slot for, sit in its
//! `leading` slot after the badge — where the React map page put them
//! (`App.tsx`: `<Footer social={false} leading={<SocialLinks …/>} />`);
//! they are the React `icons.tsx` SVGs, inline, so the brands keep their
//! colours and the service serves no extra file.

use leptos::prelude::*;
use leptos_i18n::td_string;
use palicanon_ui_lib::{EnvBadge, Footer, FooterLink};

use crate::app::BADGE;
use crate::catalogue::{Strings, strings};
use crate::pages::PageKind;

/// The App Store link from the footer, tagged with the React map's
/// campaign (`appStoreUrl("PaliWebMap")`).
pub(crate) const APP_STORE: &str =
    "https://apps.apple.com/app/apple-store/id6760995639?pt=128678815&ct=PaliWebMap&mt=8";
/// The `@daily_sutta` account.
pub(crate) const INSTAGRAM: &str = "https://www.instagram.com/daily_sutta";
/// The Ko-fi page.
pub(crate) const KOFI: &str = "https://ko-fi.com/tipitaka";

/// The footer, handed to the shell's `footer` slot.
#[component]
pub fn SiteFooter() -> impl IntoView {
    let t = strings();
    move || {
        view! {
            <Footer
                links=links(t)
                credits=Vec::new()
                leading=move || {
                    view! {
                        <div class="flex items-center gap-4">
                            <EnvBadge env=BADGE.env sha=BADGE.sha />
                            <SocialLinks t=t />
                        </div>
                    }
                }
            />
        }
    }
}

/// The five pages, in the React footer's order, labelled in `t`'s language.
fn links(t: Strings) -> Vec<FooterLink> {
    PageKind::ALL
        .into_iter()
        .map(|kind| FooterLink {
            label: kind.label(t.locale()),
            href: kind.path(),
        })
        .collect()
}

/// The three outbound icons, each named by its label and opening in a new
/// tab (the React `SocialLinks`, size 22, `gap-4`).
#[component]
fn SocialLinks(t: Strings) -> impl IntoView {
    let locale = t.locale();
    let item = |href: &'static str, label: &'static str, icon: AnyView| {
        view! {
            <li>
                <a
                    href=href
                    target="_blank"
                    rel="noopener"
                    aria-label=label
                    title=label
                    class="block rounded-md transition hover:-translate-y-0.5 hover:opacity-90"
                >
                    {icon}
                </a>
            </li>
        }
    };
    view! {
        <ul class="flex items-center gap-4">
            {item(
                APP_STORE,
                td_string!(locale, footer_app),
                view! { <AppStoreIcon size=22 /> }.into_any(),
            )}
            {item(
                INSTAGRAM,
                td_string!(locale, footer_instagram),
                view! { <InstagramIcon size=22 /> }.into_any(),
            )}
            {item(KOFI, td_string!(locale, footer_kofi), view! { <KofiIcon size=22 /> }.into_any())}
        </ul>
    }
}

/// The App Store mark (the React `AppStoreIcon`).
#[component]
pub(crate) fn AppStoreIcon(size: u32) -> impl IntoView {
    view! {
        <svg width=size height=size viewBox="0 0 22 22" aria-hidden="true">
            <defs>
                <linearGradient id="appstore-blue" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0" stop-color="#1fb1ff" />
                    <stop offset="1" stop-color="#0a6cf5" />
                </linearGradient>
            </defs>
            <rect width="22" height="22" rx="5.5" fill="url(#appstore-blue)" />
            <g stroke="#fff" stroke-width="1.6" stroke-linecap="round" fill="none">
                <path d="M6.2 15.2 11 6.6l4.8 8.6" />
                <path d="M8 12.4h6" />
                <path d="M4.6 15.2h1.3M16.1 15.2h1.3" />
            </g>
        </svg>
    }
}

/// The Instagram mark (the React `InstagramIcon`).
#[component]
pub(crate) fn InstagramIcon(size: u32) -> impl IntoView {
    view! {
        <svg width=size height=size viewBox="0 0 22 22" aria-hidden="true">
            <defs>
                <radialGradient id="ig-grad" cx="0.3" cy="1.07" r="1.2">
                    <stop offset="0" stop-color="#fdf497" />
                    <stop offset="0.1" stop-color="#fdf497" />
                    <stop offset="0.5" stop-color="#fd5949" />
                    <stop offset="0.75" stop-color="#d6249f" />
                    <stop offset="1" stop-color="#285AEB" />
                </radialGradient>
            </defs>
            <rect width="22" height="22" rx="5.5" fill="url(#ig-grad)" />
            <g fill="none" stroke="#fff" stroke-width="1.6" stroke-linecap="round">
                <rect x="5" y="5" width="12" height="12" rx="3.5" />
                <circle cx="11" cy="11" r="2.9" />
                <path d="M14.6 7.4h.01" stroke-width="2" />
            </g>
        </svg>
    }
}

/// The Ko-fi mark (the React `KofiIcon`).
#[component]
pub(crate) fn KofiIcon(size: u32) -> impl IntoView {
    view! {
        <svg width=size height=size viewBox="0 0 22 22" aria-hidden="true">
            <rect width="22" height="22" rx="5.5" fill="#ff5e5b" />
            <g
                fill="none"
                stroke="#fff"
                stroke-width="1.6"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <path d="M5.5 7.5h8.5v4.2a3.3 3.3 0 0 1-3.3 3.3H8.8a3.3 3.3 0 0 1-3.3-3.3V7.5Z" />
                <path d="M14 8.8h1a1.7 1.7 0 0 1 0 3.4h-1" />
            </g>
        </svg>
    }
}

/// These tests render markup on the server: they need `ssr`.
#[cfg(all(test, feature = "ssr"))]
mod tests {
    use palicanon_ui_lib::{AppShell, Language};

    use super::*;

    fn render() -> String {
        Owner::new().with(|| view! { <SiteFooter /> }.to_html())
    }

    /// Without the hydration markers (`<!--…-->`) between elements.
    fn plain(html: &str) -> String {
        let mut plain = html.to_owned();
        while let Some(start) = plain.find("<!--") {
            let end = start + plain[start..].find("-->").expect("comment end") + 3;
            plain.replace_range(start..end, "");
        }
        plain
    }

    /// The opening tag of the last `<div` before `at`.
    fn div_before(html: &str, at: usize) -> &str {
        let start = html[..at].rfind("<div").expect(html);
        &html[start..start + html[start..].find('>').expect(html)]
    }

    /// The opening tag of the footer's row, its first `<div`.
    fn row(html: &str) -> &str {
        let start = html.find("<div").expect(html);
        &html[start..start + html[start..].find('>').expect(html)]
    }

    /// `href` as the markup writes it (`&` escaped).
    fn href(url: &str) -> String {
        format!(r#"href="{}""#, url.replace('&', "&amp;"))
    }

    #[test]
    fn the_footer_lists_the_five_pages_in_order() {
        let html = render();
        let nav = &html[html.find("<nav").expect(&html)..html.find("</nav>").expect(&html)];
        let mut last = 0;
        for (href, label) in [
            ("/acknowledgements", "Acknowledgements"),
            ("/terms", "Terms"),
            ("/privacy", "Privacy"),
            ("/legal", "Legal"),
            ("/support", "Support"),
        ] {
            let at = nav.find(&format!(r#"<a href="{href}""#)).expect(nav);
            assert!(at > last, "{nav}");
            last = at;
            assert!(nav.contains(&format!(">{label}</a>")), "{nav}");
        }
        assert_eq!(nav.matches("<a ").count(), 5, "exactly five links: {nav}");
    }

    #[test]
    fn the_footer_labels_follow_the_language() {
        for (language, labels) in [
            (
                Language::Fr,
                [
                    "Crédits",
                    "Conditions",
                    "Confidentialité",
                    "Légal",
                    "Soutenir",
                ],
            ),
            (
                Language::Vi,
                [
                    "Tri ân",
                    "Điều khoản",
                    "Quyền riêng tư",
                    "Pháp lý",
                    "Ủng hộ",
                ],
            ),
        ] {
            let html = Owner::new().with(|| {
                palicanon_ui_lib::provide_language().set(language);
                view! { <SiteFooter /> }.to_html()
            });
            for label in labels {
                assert!(html.contains(&format!(">{label}</a>")), "{html}");
            }
        }
    }

    /// The three outbound icons, named, in a new tab, after the badge.
    #[test]
    fn the_footer_has_the_three_social_icons_after_the_badge() {
        let html = render();
        let badge = html
            .find(&format!(r#"title="{} · {}""#, BADGE.env, BADGE.sha))
            .expect(&html);
        let mut last = badge;
        for (url, label) in [
            (APP_STORE, "Get the iPhone app"),
            (INSTAGRAM, "Follow @daily_sutta on Instagram"),
            (KOFI, "Support on Ko-fi"),
        ] {
            let at = html.find(&href(url)).expect(&html);
            assert!(at > last, "{html}");
            last = at;
            let tag = &html[at..at + html[at..].find('>').expect(&html)];
            assert!(tag.contains(&format!(r#"aria-label="{label}""#)), "{tag}");
            assert!(tag.contains(r#"target="_blank""#), "{tag}");
        }
        assert!(last < html.find("<nav").expect(&html), "{html}");
    }

    /// No credits: the footer renders no credits list (the one `<ul>` is
    /// the icons').
    #[test]
    fn the_footer_has_no_credits() {
        let html = render();
        assert_eq!(html.matches("<ul").count(), 1, "{html}");
    }

    /// The shell's slot owns the wrapper: no second one here.
    #[test]
    fn the_footer_adds_no_wrapper_of_its_own() {
        let html = render();
        assert!(html.starts_with("<footer"), "{html}");
        assert!(!html.contains("mt-auto"), "{html}");
        assert!(!html.contains("min-h-dvh"), "{html}");
    }

    /// Frame, short page: one line of content, the footer still in the
    /// shell's `mt-auto` slot, the last child of the page column that
    /// fills the screen (`flex-1` in a `min-h-screen` column) — so it sits
    /// at the screen's bottom.
    #[test]
    fn a_short_page_has_the_footer_at_the_bottom_of_the_screen() {
        let html = plain(&Owner::new().with(|| {
            view! {
                <AppShell footer=|| {
                    view! { <SiteFooter /> }
                }>
                    <p>"one line"</p>
                </AppShell>
            }
            .to_html()
        }));
        assert!(
            html.contains(r#"<div class="flex min-h-screen flex-col">"#),
            "{html}"
        );
        let line = html.find("one line").expect(&html);
        let slot = html.find(r#"<div class="mt-auto"><footer"#).expect(&html);
        assert!(line < slot, "{html}");
        assert!(div_before(&html, line).contains("flex-1"), "{html}");
    }

    /// Frame, long page: 200 paragraphs, the footer after the last one, in
    /// the shell's slot, once.
    #[test]
    fn a_long_page_has_the_footer_after_its_content() {
        let html = plain(&Owner::new().with(|| {
            view! {
                <AppShell footer=|| {
                    view! { <SiteFooter /> }
                }>{(0..200).map(|n| view! { <p>{format!("line {n}")}</p> }).collect_view()}</AppShell>
            }
            .to_html()
        }));
        let last = html.find("line 199").expect(&html);
        let footer = html.find("<footer").expect(&html);
        assert!(last < footer, "{html}");
        assert!(html.contains(r#"<div class="mt-auto"><footer"#), "{html}");
        assert_eq!(html.matches("<footer").count(), 1, "{html}");
    }

    /// Frame, phone (below `sm`): one centred column, the badge and icons
    /// first, the links under them, wrapping.
    #[test]
    fn on_a_phone_the_footer_is_one_centred_column() {
        let html = plain(&render());
        let leading = html.find("<svg").expect(&html);
        let row = row(&html);
        assert!(row.contains("flex-col items-center"), "{row}");
        let nav = html.find("<nav").expect(&html);
        assert!(leading < nav, "{html}");
        let nav_tag = &html[nav..nav + html[nav..].find('>').expect(&html)];
        assert!(nav_tag.contains("flex-wrap justify-center"), "{nav_tag}");
    }

    /// Frame, desktop (from `sm`): a three-column grid, the badge and icons
    /// at the left in the first column, the links centred in the second.
    #[test]
    fn on_a_desktop_the_badge_is_left_and_the_links_centred() {
        let html = plain(&render());
        let row = row(&html);
        assert!(row.contains("sm:grid sm:grid-cols-[1fr_auto_1fr]"), "{row}");
        let badge = html.find(r#"title=""#).expect(&html);
        let leading_div = html[..badge]
            .rmatch_indices("<div")
            .map(|(at, _)| &html[at..at + html[at..].find('>').expect(&html)])
            .find(|tag| tag.contains("sm:col-start-1"))
            .expect(&html);
        assert!(
            leading_div.contains("sm:justify-self-start"),
            "{leading_div}"
        );
        let nav = html.find("<nav").expect(&html);
        let nav_tag = &html[nav..nav + html[nav..].find('>').expect(&html)];
        assert!(nav_tag.contains("sm:col-start-2"), "{nav_tag}");
    }
}
