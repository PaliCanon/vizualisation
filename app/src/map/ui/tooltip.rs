//! The tooltip (App.tsx lines 558–566): the text under the pointer, its reference and its
//! number of links, a card 12 px below and right of the pointer.

use leptos::prelude::*;

use crate::catalogue::strings;
use crate::map::model::TextRef;
use crate::map::ui::text_ref::SuttaRef;
use crate::map::ui::{CARD, links_label};

/// The text under the pointer and where the pointer is, in the page's pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Hover {
    /// The text's reference.
    pub reference: TextRef,
    /// How many links it has in the view.
    pub links: usize,
    /// The pointer's x (`clientX`).
    pub x: f64,
    /// The pointer's y (`clientY`).
    pub y: f64,
}

/// The card's `left` and `top` for a pointer at `x`, `y` (App.tsx line 562,
/// `{ left: hover.x + 12, top: hover.y + 12 }`).
pub fn tooltip_position(x: f64, y: f64) -> (String, String) {
    (format!("{}px", x + 12.0), format!("{}px", y + 12.0))
}

/// The tooltip, while a text is under the pointer.
#[component]
pub fn Tooltip(
    /// The text under the pointer, if any.
    #[prop(into)]
    hover: Signal<Option<Hover>>,
) -> impl IntoView {
    let t = strings();
    move || {
        hover.get().map(|hover| {
            let (left, top) = tooltip_position(hover.x, hover.y);
            let links = hover.links;
            view! {
                <div
                    class=format!(
                        "pointer-events-none fixed z-40 max-w-[280px] px-2.5 py-1.5 text-[13px] {CARD}",
                    )
                    style:left=left
                    style:top=top
                >
                    <SuttaRef reference=hover.reference />
                    <span class="block text-subtle">{move || links_label(links, t.locale())}</span>
                </div>
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_sits_below_right_of_the_pointer() {
        assert_eq!(
            tooltip_position(100.0, 50.5),
            ("112px".to_owned(), "62.5px".to_owned())
        );
    }

    #[cfg(feature = "ssr")]
    fn render_tooltip(language: palicanon_ui_lib::Language, links: usize) -> String {
        use crate::map::ui::test_support::{plain, render};

        let hover = Hover {
            reference: TextRef {
                acronym: "MN 10".to_owned(),
                pali: "Satipaṭṭhāna".to_owned(),
                title: "Mindfulness Meditation".to_owned(),
            },
            links,
            x: 100.0,
            y: 50.0,
        };
        plain(&render(
            language,
            move || view! { <Tooltip hover=Some(hover) /> },
        ))
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_tooltip_counts_the_links_in_each_language() {
        use palicanon_ui_lib::Language;

        // App.tsx:564 `t("tooltip.links", { count })`: the count, then the word
        for (language, many, one) in [
            (Language::En, "3 links", "1 link"),
            (Language::Fr, "3 liens", "1 lien"),
            (Language::Vi, "3 liên kết", "1 liên kết"),
        ] {
            let html = render_tooltip(language, 3);
            assert!(html.contains(many), "{language:?}: {html}");
            assert!(
                html.contains(">MN 10<") && html.contains("Satipaṭṭhāna"),
                "{html}"
            );
            assert!(render_tooltip(language, 1).contains(one), "{language:?}");
        }
        let html = render_tooltip(Language::En, 3);
        assert!(html.contains("left:112px;top:62px"), "{html}");
        assert!(html.contains("pointer-events-none fixed"), "{html}");
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn no_text_under_the_pointer_no_tooltip() {
        use palicanon_ui_lib::Language;

        use crate::map::ui::test_support::{plain, render};

        let none: Option<Hover> = None;
        let html = plain(&render(Language::En, || view! { <Tooltip hover=none /> }));
        assert!(!html.contains("<div"), "{html}");
    }
}
