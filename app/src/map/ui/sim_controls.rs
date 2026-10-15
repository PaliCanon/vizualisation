//! The vectorial view's controls (`src/components/SimControls.tsx`): the similarity
//! threshold, how many links and lone texts it gives, and "hide texts without a neighbour".
//!
//! The slider is ui-lib's `Slider`, which counts in whole numbers: the threshold is held in
//! hundredths (0.42 → 42), the React slider's 0.01 step (a ui-lib follow-up: a decimal
//! slider, its value shown by the caller's format).

use leptos::prelude::*;
use leptos_i18n::td_string;
use palicanon_ui_lib::{Slider, language_signal};

use crate::catalogue::strings;
use crate::map::ui::{interpolate, number};

/// `x` in hundredths, rounded; negative or not a number → 0.
pub fn hundredths(x: f64) -> u32 {
    to_u32((x * 100.0).round())
}

/// A whole, non-negative `f64` as a `u32`; anything else is clamped (NaN → 0).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to u32's range first; the casts only drop a zero fraction"
)]
fn to_u32(x: f64) -> u32 {
    if x.is_nan() {
        return 0;
    }
    x.clamp(0.0, f64::from(u32::MAX)) as u32
}

/// The slider's range in hundredths for the view's `[min, max]` (`SimInfo`): inside it,
/// `min` rounded up, `max` down, so every value the slider gives is in the pairs' range.
pub fn slider_range(min: f64, max: f64) -> (u32, u32) {
    // Rounded first, so 0.6 (59.999…) is 60, not 59 or 61.
    let up = |x: f64| to_u32(((x * 1e6).round() / 1e4).ceil());
    let down = |x: f64| to_u32(((x * 1e6).round() / 1e4).floor());
    (up(min), down(max))
}

/// A threshold in hundredths as the page shows it, `0.42` (SimControls.tsx line 20,
/// `value.toFixed(2)`).
pub fn threshold_label(value: u32) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}

/// The threshold slider, its statistics and the lone texts' switch.
#[component]
pub fn SimControls(
    /// The view's lowest similarity (`SimInfo::min`).
    min: f64,
    /// Its highest (`SimInfo::max`).
    max: f64,
    /// The threshold shown, in hundredths, written while the slider moves (the wiring applies
    /// it once it settles, App.tsx lines 327–337).
    value: RwSignal<u32>,
    /// How many links the applied threshold gives.
    #[prop(into)]
    links: Signal<usize>,
    /// How many texts have none.
    #[prop(into)]
    alone: Signal<usize>,
    /// Texts without a neighbour are hidden.
    #[prop(into)]
    hide_alone: Signal<bool>,
    /// The switch was flipped: its new state.
    on_hide_alone: Callback<bool>,
) -> impl IntoView {
    let t = strings();
    let (low, high) = slider_range(min, max);
    let stats = move || {
        let language = language_signal().get();
        interpolate(
            t.locale(),
            "sim_stats",
            &[
                ("links", &number(links.get(), language)),
                ("alone", &number(alone.get(), language)),
            ],
        )
    };
    view! {
        <div class="mt-3 px-2">
            <div class="block text-[13px] text-ink">
                <span class="flex justify-between">
                    {move || td_string!(t.locale(), sim_threshold)}
                    <span class="font-medium tabular-nums text-gold">
                        {move || threshold_label(value.get())}
                    </span>
                </span>
                <Slider
                    value=value
                    min=low
                    max=high
                    label=Signal::derive(move || td_string!(t.locale(), sim_threshold).to_owned())
                    show_value=false
                    layout="mt-2"
                />
            </div>
            <p class="mt-1 text-xs text-subtle">{stats}</p>
            <label class="mt-2 flex cursor-pointer items-center gap-2 text-[13px] text-ink">
                <input
                    type="checkbox"
                    checked=move || hide_alone.get()
                    prop:checked=move || hide_alone.get()
                    on:change=move |ev| on_hide_alone.run(event_target_checked(&ev))
                    class="accent-gold"
                />
                {move || td_string!(t.locale(), sim_hide_alone)}
            </label>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_threshold_is_counted_in_hundredths() {
        // SimControls.tsx:26 `step={0.01}`, line 20 `value.toFixed(2)`
        assert_eq!(hundredths(0.42), 42);
        assert_eq!(hundredths(0.617), 62);
        assert_eq!(hundredths(-1.0), 0);
        assert_eq!(hundredths(f64::NAN), 0);
        assert_eq!(slider_range(0.613, 0.951), (62, 95));
        assert_eq!(slider_range(0.6, 0.9), (60, 90));
        assert_eq!(threshold_label(42), "0.42");
        assert_eq!(threshold_label(5), "0.05");
        assert_eq!(threshold_label(100), "1.00");
    }

    /// SimControls.tsx:19 `t("sim.threshold")`, 33 `t("sim.stats", {links, alone})`, 37
    /// `t("sim.hide_alone")`, in each language.
    #[cfg(feature = "ssr")]
    const WORDS: [(palicanon_ui_lib::Language, [&str; 3]); 3] = [
        (
            palicanon_ui_lib::Language::En,
            [
                "Similarity threshold",
                "12,345 links · 7 texts without a neighbour",
                "Hide texts without a neighbour",
            ],
        ),
        (
            palicanon_ui_lib::Language::Fr,
            [
                "Seuil de similarité",
                "12\u{202f}345 liens · 7 textes sans voisin",
                "Masquer les textes sans voisin",
            ],
        ),
        (
            palicanon_ui_lib::Language::Vi,
            [
                "Ngưỡng tương đồng",
                "12.345 liên kết · 7 bài kinh không có láng giềng",
                "Ẩn các bài kinh không có láng giềng",
            ],
        ),
    ];

    #[cfg(feature = "ssr")]
    #[test]
    fn the_controls_are_labelled_and_counted_in_each_language() {
        use crate::map::ui::test_support::{plain, render, tag_around};

        for (language, words) in WORDS {
            let html = plain(&render(language, || {
                let value = RwSignal::new(70);
                view! {
                    <SimControls
                        min=0.613
                        max=0.951
                        value=value
                        links=12_345
                        alone=7
                        hide_alone=true
                        on_hide_alone=Callback::new(|_| {})
                    />
                }
            }));
            for word in words {
                assert!(html.contains(word), "{language:?} {word}: {html}");
            }
            assert!(html.contains(">0.70<"), "{html}");
            let range = tag_around(&html, r#"type="range""#);
            assert!(
                range.contains(r#"min="62""#) && range.contains(r#"max="95""#),
                "{range}"
            );
            let checkbox = tag_around(&html, r#"type="checkbox""#);
            assert!(checkbox.contains("checked"), "{checkbox}");
        }
    }
}
