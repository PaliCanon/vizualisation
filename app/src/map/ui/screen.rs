//! The map screen's layout (App.tsx lines 406–568), inside ui-lib's `AppShell` (which now
//! holds the React header's brand, language and settings): the search bar on top; under it a
//! row — the legend, the map, the text panel —; the chips over the map; the tooltip.
//!
//! The screen is one viewport tall (less the shell's phone top bar, `h-14`), so the map fills
//! it and the shell's footer follows below; the legend's list and the panel scroll inside
//! it. On a phone (below `md`, the React page's `useMediaQuery("(max-width: 767px)")`) the
//! legend is a drawer over the map, closed on every page load, with a backdrop that closes
//! it; on a desktop it is an aside beside the map, open or closed as the reader left it
//! (`viz_legend`). Both are decided by CSS, so the server's page is right on either before
//! any script runs. The panel covers the map below `lg`, and sits beside it above.
//!
//! The screen knows no engine: the canvas, the search, the chips, the legend and the panel
//! are slots the wiring fills.

use leptos::prelude::*;
use leptos_i18n::td_string;

use crate::catalogue::strings;

/// The phone's breakpoint, in CSS pixels: below it the legend is a drawer (Tailwind's `md`).
pub const NARROW_BELOW: f64 = 768.0;

/// The legend's open flags after its toggle is pressed (App.tsx line 492): on a phone the
/// drawer flips, on a desktop the aside does. Returns `(legend_open, drawer_open)`.
pub fn legend_toggled(narrow: bool, legend_open: bool, drawer_open: bool) -> (bool, bool) {
    if narrow {
        (legend_open, !drawer_open)
    } else {
        (!legend_open, drawer_open)
    }
}

/// The legend aside's classes (App.tsx line 465): a drawer over the map on a phone, shown
/// while `drawer_open`; an aside in the row on a desktop, shown while `legend_open`. The
/// drawer starts under the chips' row (`top-12`: the row is 42 px), so its toggle, the first
/// chip, stays in reach to close it (006 review R4; the React drawer covered it).
pub fn legend_classes(legend_open: bool, drawer_open: bool) -> String {
    let phone = if drawer_open { "flex" } else { "hidden" };
    let desktop = if legend_open { "md:flex" } else { "md:hidden" };
    format!(
        "absolute top-12 bottom-0 left-0 z-30 {phone} w-64 max-w-[85vw] shrink-0 flex-col border-r border-line bg-page md:static {desktop}"
    )
}

/// The media query of a phone's page, below [`NARROW_BELOW`] (the React page's
/// `useMediaQuery("(max-width: 767px)")`).
pub const NARROW_QUERY: &str = "(max-width: 767px)";

/// Whether the page is a phone's, following the window: `false` on the server and in the
/// browser's first render (the desktop's, so hydration matches), then the media query's
/// answer once hydrated, and again at each change of it (a resize, a rotation).
fn narrow_signal() -> RwSignal<bool> {
    let narrow = RwSignal::new(false);
    #[cfg(feature = "browser")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        let Some(query) = window().match_media(NARROW_QUERY).ok().flatten() else {
            return narrow;
        };
        // After hydrating: the first render was the desktop's on both sides.
        {
            let query = query.clone();
            Effect::new(move |_| narrow.set(query.matches()));
        }
        let listener = {
            let query = query.clone();
            Closure::<dyn FnMut()>::new(move || {
                let _ = narrow.try_set(query.matches());
            })
        };
        let added = query
            .add_event_listener_with_callback("change", listener.as_ref().unchecked_ref())
            .is_ok();
        if added {
            let kept = StoredValue::new_local(Some((query, listener)));
            on_cleanup(move || {
                if let Some(Some((query, listener))) = kept.try_update_value(Option::take) {
                    let _ = query.remove_event_listener_with_callback(
                        "change",
                        listener.as_ref().unchecked_ref(),
                    );
                }
            });
        }
    }
    narrow
}

/// The map screen.
#[component]
pub fn MapScreen(
    /// The canvas (the wiring's `EngineView`).
    #[prop(into)]
    map: ViewFn,
    /// The search box.
    #[prop(into)]
    search: ViewFn,
    /// Over the map, after the legend's toggle: the views and texts chips.
    #[prop(into)]
    chips: ViewFn,
    /// The legend.
    #[prop(into)]
    legend: ViewFn,
    /// The legend is open on a desktop (`viz_legend`'s `open`).
    legend_open: RwSignal<bool>,
    /// The legend's drawer is open on a phone; closed when a page loads.
    drawer_open: RwSignal<bool>,
    /// The text panel.
    #[prop(into)]
    panel: ViewFn,
    /// The panel is shown: a text is open and the panel not closed.
    #[prop(into)]
    panel_open: Signal<bool>,
    /// Over the map: the loading or the no-GPU message.
    #[prop(optional, into)]
    overlay: Option<ViewFn>,
    /// The tooltip.
    #[prop(optional, into)]
    tooltip: Option<ViewFn>,
) -> impl IntoView {
    view! {
        <div class="flex h-[calc(100dvh-3.5rem)] min-h-0 flex-col sm:h-dvh">
            <div class="relative z-[35] shrink-0 border-b border-line">
                <div class="mx-auto w-full max-w-[40rem] px-4 py-3">{search.run()}</div>
            </div>
            <div class="relative flex min-h-0 flex-1">
                <LegendAside legend=legend legend_open=legend_open drawer_open=drawer_open />
                <main class="relative min-w-0 flex-1 overflow-hidden">
                    {map.run()}
                    <MapChips chips=chips legend_open=legend_open drawer_open=drawer_open />
                    {overlay.map(|overlay| overlay.run())}
                </main>
                {move || {
                    panel_open
                        .get()
                        .then(|| {
                            view! {
                                <aside class="absolute inset-y-0 right-0 z-30 w-full shrink-0 border-l border-line bg-page sm:w-80 lg:static">
                                    {panel.run()}
                                </aside>
                            }
                        })
                }}
            </div>
            {tooltip.map(|tooltip| tooltip.run())}
        </div>
    }
}

/// The legend: a drawer on a phone, with a backdrop that closes it; an aside on a desktop
/// (App.tsx lines 460–489).
#[component]
fn LegendAside(
    legend: ViewFn,
    legend_open: RwSignal<bool>,
    drawer_open: RwSignal<bool>,
) -> impl IntoView {
    let t = strings();
    view! {
        {move || {
            drawer_open
                .get()
                .then(|| {
                    view! {
                        <button
                            type="button"
                            aria-label=move || td_string!(t.locale(), legend_toggle)
                            on:click=move |_| drawer_open.set(false)
                            class="absolute inset-0 z-20 cursor-default bg-black/40 md:hidden"
                        ></button>
                    }
                })
        }}
        <aside class=move || legend_classes(legend_open.get(), drawer_open.get())>
            <div class="min-h-0 flex-1 overflow-y-auto">{legend.run()}</div>
        </aside>
    }
}

/// The chips over the map, centred like palicanon's filter chips: the legend's toggle, then
/// the wiring's (App.tsx lines 485–517). Above the phone drawer's backdrop (`z-20`), and
/// above the drawer, which starts under them ([`legend_classes`]): with the drawer open,
/// its toggle still closes it.
#[component]
fn MapChips(
    chips: ViewFn,
    legend_open: RwSignal<bool>,
    drawer_open: RwSignal<bool>,
) -> impl IntoView {
    let t = strings();
    // The server renders the desktop's state; a phone's drawer starts closed anyway.
    let narrow = narrow_signal();
    let pressed = move || {
        if narrow.get() {
            drawer_open.get()
        } else {
            legend_open.get()
        }
    };
    let toggle = move |_| {
        let (legend, drawer) = legend_toggled(
            narrow.get_untracked(),
            legend_open.get_untracked(),
            drawer_open.get_untracked(),
        );
        legend_open.set(legend);
        drawer_open.set(drawer);
    };
    let class = move || {
        let state = if pressed() {
            "border-gold/60 text-heading"
        } else {
            "border-line text-muted hover:border-card-edge"
        };
        format!(
            "grid size-[30px] shrink-0 cursor-pointer place-items-center rounded-full border transition hover:text-heading {state}"
        )
    };
    view! {
        <div class="pointer-events-none absolute inset-x-0 top-0 z-[25] flex justify-center px-4 pt-3">
            <div class="pointer-events-auto flex flex-wrap items-center justify-center gap-2">
                <button
                    type="button"
                    on:click=toggle
                    aria-label=move || td_string!(t.locale(), legend_toggle)
                    title=move || td_string!(t.locale(), legend_toggle)
                    aria-pressed=move || pressed().to_string()
                    class=class
                >
                    <PanelIcon />
                </button>
                {chips.run()}
            </div>
        </div>
    }
}

/// The legend's toggle icon: a panel with its left column (the React page's `PanelIcon`,
/// not in ui-lib v0.3.0).
#[component]
fn PanelIcon() -> impl IntoView {
    view! {
        <svg
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <rect x="3" y="4" width="18" height="16" rx="2.5" />
            <path d="M9 4v16" />
        </svg>
    }
}

/// The map's data did not arrive (a failed request, a file that does not parse): said over
/// the map, in place of "Loading the map…" (which the React page kept for ever).
#[component]
pub fn MapError() -> impl IntoView {
    let t = strings();
    view! {
        <div role="alert" class="absolute inset-0 grid place-items-center p-6">
            <div class="max-w-md text-center">
                <p class="font-serif text-xl italic">
                    {move || td_string!(t.locale(), map_error_title)}
                </p>
                <p class="mt-3 text-sm text-muted">
                    {move || td_string!(t.locale(), map_error_body)}
                </p>
            </div>
        </div>
    }
}

/// "Loading the map…", over the map until its data is in (App.tsx line 526).
#[component]
pub fn MapLoading() -> impl IntoView {
    let t = strings();
    view! {
        <p class="absolute inset-0 grid place-items-center font-serif text-xl italic text-muted">
            {move || td_string!(t.locale(), loading)}
        </p>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_toggle_flips_the_drawer_on_a_phone_the_aside_on_a_desktop() {
        // App.tsx:492 `onClick={() => setLegendOpen((o) => !o)}`, one flag per layout here
        assert_eq!(legend_toggled(true, true, false), (true, true));
        assert_eq!(legend_toggled(true, true, true), (true, false));
        assert_eq!(legend_toggled(false, true, false), (false, false));
        assert_eq!(legend_toggled(false, false, true), (true, true));
    }

    #[test]
    fn the_phone_query_is_the_breakpoint() {
        assert_eq!(
            NARROW_QUERY,
            format!("(max-width: {}px)", NARROW_BELOW - 1.0)
        );
    }

    #[cfg(feature = "ssr")]
    mod frame {
        use palicanon_ui_lib::Language;

        use super::super::*;
        use crate::map::ui::test_support::{classes, plain, render, tag_around};

        /// The screen with stand-ins in its slots: the panel holds `panel_rows` rows.
        fn screen(legend_open: bool, drawer_open: bool, panel_rows: Option<usize>) -> String {
            plain(&render(Language::En, move || {
                let legend_open = RwSignal::new(legend_open);
                let drawer_open = RwSignal::new(drawer_open);
                view! {
                    <MapScreen
                        map=|| view! { <canvas id="map-canvas"></canvas> }
                        search=|| view! { <form id="search-slot"></form> }
                        chips=|| view! { <span id="chips-slot"></span> }
                        legend=|| view! { <div id="legend-slot"></div> }
                        legend_open=legend_open
                        drawer_open=drawer_open
                        panel=move || {
                            view! {
                                <div id="panel-slot" class="relative flex h-full flex-col">
                                    <div class="min-h-0 flex-1 overflow-y-auto">
                                        {(0..panel_rows.unwrap_or(0))
                                            .map(|k| view! { <p>{k}</p> })
                                            .collect_view()}
                                    </div>
                                </div>
                            }
                        }
                        panel_open=panel_rows.is_some()
                    />
                }
            }))
        }

        fn has(tag: &str, wanted: &[&str]) {
            let got = classes(tag);
            for class in wanted {
                assert!(got.contains(class), "{class}: {tag}");
            }
        }

        fn aside_of(html: &str, slot: &str) -> String {
            let at = html.find(slot).expect(html);
            let open = html[..at].rfind("<aside").expect(html);
            tag_around(html, &html[open..at])
        }

        /// Short page (no text open): the screen is the viewport's height less the phone's
        /// top bar, the map fills the row, no panel; the shell's footer follows the screen.
        #[test]
        fn frame_short_page_the_screen_is_one_viewport_and_the_map_fills_it() {
            let html = screen(true, false, None);
            let root = tag_around(&html, "<div");
            has(
                &root,
                &["flex", "flex-col", "h-[calc(100dvh-3.5rem)]", "sm:h-dvh"],
            );
            let main = tag_around(&html, "<main");
            has(&main, &["relative", "min-w-0", "flex-1", "overflow-hidden"]);
            assert!(
                html.find("search-slot") < html.find("<main"),
                "search on top: {html}"
            );
            assert!(
                html.contains("map-canvas") && html.contains("chips-slot"),
                "{html}"
            );
            assert!(
                !html.contains("panel-slot"),
                "no text open, no panel: {html}"
            );
            assert!(!html.contains("min-h-screen"), "the shell's own: {html}");
        }

        /// Long page (a text with many neighbours): the panel stays in the row and scrolls
        /// inside; the screen keeps its height.
        #[test]
        fn frame_long_page_the_panel_scrolls_inside_the_screen() {
            let html = screen(true, false, Some(300));
            let root = tag_around(&html, "<div");
            has(&root, &["h-[calc(100dvh-3.5rem)]", "sm:h-dvh"]);
            let row = tag_around(&html, "relative flex min-h-0 flex-1");
            has(&row, &["min-h-0", "flex-1"]);
            let panel = aside_of(&html, "panel-slot");
            has(&panel, &["inset-y-0", "border-l", "bg-page"]);
            assert!(html.contains("min-h-0 flex-1 overflow-y-auto"), "{html}");
            let legend = aside_of(&html, "legend-slot");
            let list = tag_around(&html[html.find(&legend).expect(&html)..], "<div");
            has(&list, &["min-h-0", "flex-1", "overflow-y-auto"]);
        }

        /// Phone: the legend is a drawer over the map, closed until its toggle opens it
        /// (whatever the desktop's choice), with a backdrop; the panel covers the map.
        #[test]
        fn frame_phone_the_legend_is_a_drawer_with_a_backdrop() {
            let closed = screen(true, false, Some(1));
            let drawer = aside_of(&closed, "legend-slot");
            has(
                &drawer,
                &[
                    "absolute",
                    "top-12",
                    "bottom-0",
                    "left-0",
                    "z-30",
                    "w-64",
                    "max-w-[85vw]",
                    "hidden",
                ],
            );
            assert!(
                !closed.contains("bg-black/40"),
                "closed, no backdrop: {closed}"
            );
            let panel = aside_of(&closed, "panel-slot");
            has(&panel, &["absolute", "right-0", "z-30", "w-full"]);
            let open = screen(false, true, None);
            has(&aside_of(&open, "legend-slot"), &["flex", "md:hidden"]);
            let backdrop = tag_around(&open, "bg-black/40");
            assert!(backdrop.starts_with("<button"), "{backdrop}");
            has(&backdrop, &["absolute", "inset-0", "z-20", "md:hidden"]);
            // 006 review R4: the chips (the legend's toggle first) above the backdrop,
            // below the drawer.
            let chips = tag_around(&open, "pointer-events-none absolute");
            has(&chips, &["z-[25]"]);
            assert!(
                open.find("bg-black/40") < open.find("aria-pressed"),
                "{open}"
            );
            assert!(
                backdrop.contains(r#"aria-label="Show / hide the collections""#),
                "{backdrop}"
            );
        }

        /// Desktop: the legend is an aside in the row, left of the map, shown as the reader
        /// left it; the panel sits right of the map.
        #[test]
        fn frame_desktop_the_legend_is_an_aside_beside_the_map() {
            let html = screen(true, false, Some(1));
            let aside = aside_of(&html, "legend-slot");
            has(&aside, &["md:static", "md:flex", "border-r", "shrink-0"]);
            assert!(!classes(&aside).contains(&"md:hidden"), "{aside}");
            let legend = html.find("legend-slot").expect(&html);
            let main = html.find("<main").expect(&html);
            let panel = html.find("panel-slot").expect(&html);
            assert!(legend < main && main < panel, "legend, map, panel: {html}");
            has(&aside_of(&html, "panel-slot"), &["sm:w-80", "lg:static"]);
            let closed = screen(false, false, None);
            has(&aside_of(&closed, "legend-slot"), &["md:hidden"]);
            let toggle = tag_around(&html, "aria-pressed");
            assert!(toggle.contains(r#"aria-pressed="true""#), "{toggle}");
        }

        /// 006 review R3: a failed fetch says so, in each language.
        #[test]
        fn the_error_message_in_each_language() {
            for (language, title) in [
                (Language::En, "The map could not load"),
                (Language::Fr, "La carte n’a pas pu se charger"),
                (Language::Vi, "Không thể tải bản đồ"),
            ] {
                let html = plain(&render(language, || view! { <MapError /> }));
                assert!(html.contains(title), "{language:?}: {html}");
                assert!(html.contains(r#"role="alert""#), "{html}");
            }
        }

        #[test]
        fn the_toggle_and_the_loading_message_in_each_language() {
            // App.tsx:495 `aria-label={t("legend.toggle")}`, 526 `t("loading")`
            for (language, toggle, loading) in [
                (
                    Language::En,
                    "Show / hide the collections",
                    "Loading the map…",
                ),
                (
                    Language::Fr,
                    "Afficher / masquer les collections",
                    "Chargement de la carte…",
                ),
                (Language::Vi, "Hiện / ẩn các bộ kinh", "Đang tải bản đồ…"),
            ] {
                let html = plain(&render(language, || {
                    view! {
                        <MapScreen
                            map=|| ()
                            search=|| ()
                            chips=|| ()
                            legend=|| ()
                            legend_open=RwSignal::new(true)
                            drawer_open=RwSignal::new(false)
                            panel=|| ()
                            panel_open=false
                            overlay=|| view! { <MapLoading /> }
                        />
                    }
                }));
                assert!(
                    html.contains(&format!(r#"aria-label="{toggle}""#)),
                    "{language:?}: {html}"
                );
                assert!(html.contains(loading), "{language:?}: {html}");
            }
        }
    }
}
