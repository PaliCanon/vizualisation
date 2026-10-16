//! The legend (`src/components/Legend.tsx`): the collections — a click colours one, the eye
//! hides it — and the link types — a click shows or hides one —, each with how many texts or
//! links it has; the vectorial view's controls under the types (its children).

use leptos::prelude::*;
use leptos_i18n::td_string;
use palicanon_ui_lib::language_signal;
use serde::{Deserialize, Serialize};

use crate::catalogue::strings;
use crate::map::draw::Scene;
use crate::map::legend::Legend as LegendState;
use crate::map::model::Graph;
use crate::map::theme::{MapTheme, NO_GROUP_COLOR};
use crate::map::ui::{number, type_label};

/// The classes of a legend heading (Legend.tsx line 6, `sectionTitle`).
pub const SECTION_TITLE: &str = "px-2 text-[11px] font-medium uppercase tracking-wider text-subtle";

/// One collection of the legend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionRow {
    /// Its id in the `viz_legend` cookie (`Group::iso`).
    pub iso: String,
    /// Its name.
    pub name: String,
    /// Its colour, `#rrggbb`.
    pub color: String,
    /// How many of its texts the texts' language has.
    pub count: usize,
}

/// One link type of the legend.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeRow {
    /// Its index in `Graph::types`.
    pub index: usize,
    /// Its name (`full`, `similar`…).
    pub name: String,
    /// Its colour in the theme, `#rrggbb` (none for an unknown type).
    pub color: Option<String>,
    /// How many links of this type the view has.
    pub count: usize,
}

/// The collections with a text in the texts' language (`available`; `None`: all), in the
/// graph's order (Legend.tsx lines 24 and 34, `counts`, `if (!counts[i]) return null`).
pub fn collection_rows(
    graph: &Graph,
    theme: &MapTheme,
    available: Option<&[bool]>,
) -> Vec<CollectionRow> {
    let mut counts = vec![0_usize; graph.langs.len()];
    for (i, node) in graph.nodes.iter().enumerate() {
        let shown = available.is_none_or(|on| on.get(i).copied().unwrap_or(false));
        if shown && let Some(count) = counts.get_mut(node.group as usize) {
            *count += 1;
        }
    }
    graph
        .langs
        .iter()
        .zip(counts)
        .enumerate()
        .filter(|(_, (_, count))| *count > 0)
        .map(|(i, (group, count))| CollectionRow {
            iso: group.iso.clone(),
            name: group.name.clone(),
            color: theme
                .groups
                .get(i)
                .map_or(NO_GROUP_COLOR, String::as_str)
                .to_owned(),
            count,
        })
        .collect()
}

/// The link types with their colour and count (Legend.tsx lines 65–75; `link_counts` from
/// `Scene::link_counts`).
pub fn type_rows(graph: &Graph, theme: &MapTheme, link_counts: &[usize]) -> Vec<TypeRow> {
    graph
        .types
        .iter()
        .enumerate()
        .map(|(index, name)| TypeRow {
            index,
            name: name.clone(),
            // TODO(006 merge): `Some(theme.link_color(name).to_owned())` (lane A's
            // `MapTheme::link_color`, 4d21574: the map's fallback for an unknown type).
            color: theme.type_color(name).map(str::to_owned),
            count: link_counts.get(index).copied().unwrap_or(0),
        })
        .collect()
}

/// A vectorial view's slider: its similarity range and the threshold applied, in hundredths.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimRange {
    /// `SimInfo::min`.
    pub min: f64,
    /// `SimInfo::max`.
    pub max: f64,
    /// The threshold, in hundredths (the slider's unit).
    pub value: u32,
}

/// Everything the legend shows of one view: the server renders it before the browser has
/// the data (so the aside is not empty until then), the browser from its own data after.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LegendRows {
    /// [`collection_rows`].
    pub collections: Vec<CollectionRow>,
    /// [`type_rows`], with the scene's links.
    pub types: Vec<TypeRow>,
    /// How many links of the first type (a vectorial view's only one): the slider's
    /// statistics.
    pub links: usize,
    /// How many texts have no link.
    pub alone: usize,
    /// A vectorial view's slider.
    pub sim: Option<SimRange>,
}

/// The legend of `graph` in `theme`, its links `scene`'s, its texts those of `available`
/// (`None`: all), its slider at `threshold` on a vectorial view.
pub fn legend_rows(
    graph: &Graph,
    theme: &MapTheme,
    scene: &Scene,
    available: Option<&[bool]>,
    threshold: Option<f64>,
) -> LegendRows {
    let counts = scene.link_counts(graph);
    LegendRows {
        collections: collection_rows(graph, theme, available),
        types: type_rows(graph, theme, &counts),
        links: counts.first().copied().unwrap_or(0),
        alone: scene.alone(),
        sim: graph.sim.as_ref().map(|sim| SimRange {
            min: sim.min,
            max: sim.max,
            value: crate::map::ui::sim_controls::hundredths(threshold.unwrap_or(sim.default)),
        }),
    }
}

/// The legend: collections, then link types and `children` (the vectorial view's controls).
#[component]
pub fn Legend(
    /// The collections shown.
    #[prop(into)]
    collections: Signal<Vec<CollectionRow>>,
    /// The link types.
    #[prop(into)]
    types: Signal<Vec<TypeRow>>,
    /// Which collections are coloured and hidden.
    #[prop(into)]
    legend: Signal<LegendState>,
    /// Per link type (`TypeRow::index`): shown.
    #[prop(into)]
    type_on: Signal<Vec<bool>>,
    /// A collection was clicked: colour or uncolour it (its iso).
    on_color: Callback<String>,
    /// A collection's eye was clicked: hide or show it (its iso).
    on_hide: Callback<String>,
    /// A link type was clicked: show or hide it (its index).
    on_type: Callback<usize>,
    /// Under the link types: the vectorial view's controls.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let t = strings();
    let rows = move || {
        collections
            .get()
            .into_iter()
            .map(|row| view! { <CollectionItem row=row legend=legend on_color=on_color on_hide=on_hide /> })
            .collect_view()
    };
    let type_rows = move || {
        types
            .get()
            .into_iter()
            .map(|row| view! { <TypeItem row=row type_on=type_on on_type=on_type /> })
            .collect_view()
    };
    view! {
        <div class="space-y-5 px-2 py-4">
            <section>
                <h2 class=SECTION_TITLE>{move || td_string!(t.locale(), legend_collections)}</h2>
                <p class="mb-2 mt-1 px-2 text-xs text-subtle">
                    {move || td_string!(t.locale(), legend_hint)}
                </p>
                <ul>{rows}</ul>
            </section>
            <section>
                <h2 class=SECTION_TITLE>{move || td_string!(t.locale(), legend_links)}</h2>
                <ul class="mt-1.5">{type_rows}</ul>
                {children.map(|children| children())}
            </section>
        </div>
    }
}

/// One collection: a click colours it, its eye hides it (Legend.tsx lines 33–59).
#[component]
fn CollectionItem(
    row: CollectionRow,
    legend: Signal<LegendState>,
    on_color: Callback<String>,
    on_hide: Callback<String>,
) -> impl IntoView {
    let t = strings();
    let iso = row.iso.clone();
    let colored = Signal::derive(move || legend.with(|l| l.colored.contains(&iso)));
    let iso = row.iso.clone();
    let shown = Signal::derive(move || legend.with(|l| !l.hidden.contains(&iso)));
    let class = move || {
        let on = if colored.get() {
            "bg-chip-hover text-heading shadow-[inset_2px_0_0_var(--ps-gold)]"
        } else {
            "text-ink"
        };
        let faded = if shown.get() { "" } else { " opacity-40" };
        format!(
            "flex cursor-pointer select-none items-center gap-2.5 rounded-lg px-2 py-1 text-[13px] transition hover:bg-chip-hover {on}{faded}"
        )
    };
    let eye_label = move || {
        if shown.get() {
            td_string!(t.locale(), legend_hide)
        } else {
            td_string!(t.locale(), legend_show)
        }
    };
    let name = row.name.clone();
    let (color_iso, hide_iso) = (row.iso.clone(), row.iso);
    view! {
        <li
            class=class
            title=move || td_string!(t.locale(), legend_color)
            on:click=move |_| on_color.run(color_iso.clone())
        >
            <span class="size-3 shrink-0 rounded-full" style:background=row.color></span>
            {row.name}
            <span class="ml-auto tabular-nums text-subtle">
                {move || number(row.count, language_signal().get())}
            </span>
            <button
                type="button"
                title=eye_label
                aria-label=move || format!("{} {name}", eye_label())
                class="flex cursor-pointer pl-1 text-subtle transition hover:text-heading"
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_hide.run(hide_iso.clone());
                }
            >
                {move || {
                    if shown.get() {
                        view! { <EyeIcon /> }.into_any()
                    } else {
                        view! { <EyeOffIcon /> }.into_any()
                    }
                }}
            </button>
        </li>
    }
}

/// One link type: a click shows or hides its links (Legend.tsx lines 65–76).
#[component]
fn TypeItem(row: TypeRow, type_on: Signal<Vec<bool>>, on_type: Callback<usize>) -> impl IntoView {
    let t = strings();
    let index = row.index;
    let class = move || {
        let on = type_on.with(|on| on.get(index).copied().unwrap_or(true));
        let faded = if on { "" } else { " opacity-40" };
        format!(
            "flex cursor-pointer select-none items-center gap-2.5 rounded-lg px-2 py-1 text-[13px] text-ink transition hover:bg-chip-hover{faded}"
        )
    };
    let name = row.name;
    view! {
        <li class=class on:click=move |_| on_type.run(index)>
            <span class="h-0.5 w-3.5 shrink-0 rounded-full" style:background=row.color></span>
            {move || type_label(&name, t.locale()).to_owned()}
            <span class="ml-auto tabular-nums text-subtle">
                {move || number(row.count, language_signal().get())}
            </span>
        </li>
    }
}

/// An open eye (the React page's `EyeIcon`, not in ui-lib v0.1.2).
#[component]
fn EyeIcon() -> impl IntoView {
    view! {
        <svg
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
            <circle cx="8" cy="8" r="2" />
        </svg>
    }
}

/// A crossed eye (the React page's `EyeOffIcon`).
#[component]
fn EyeOffIcon() -> impl IntoView {
    view! {
        <svg
            width="16"
            height="16"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
            <path d="m2.5 2.5 11 11" />
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::fixture;

    #[test]
    fn collections_are_counted_in_the_texts_language() {
        // Legend.tsx:24 `graph.nodes.forEach((n, i) => { if (!available || available[i]) counts[n[5]]!++; })`
        let g = fixture();
        let theme = MapTheme::new(false, &g);
        let rows = collection_rows(&g, &theme, None);
        let counts: Vec<(&str, usize)> = rows.iter().map(|r| (r.iso.as_str(), r.count)).collect();
        assert_eq!(counts, [("dn", 1), ("mn", 3), ("kn", 1)]);
        assert_eq!(rows[1].name, "Majjhima Nikāya");
        assert_eq!(rows[1].color, "#ff6f8e");
        // Legend.tsx:34 `if (!counts[i]) return null;`
        let available = [false, true, false, true, false];
        let rows = collection_rows(&g, &theme, Some(&available));
        let counts: Vec<(&str, usize)> = rows.iter().map(|r| (r.iso.as_str(), r.count)).collect();
        assert_eq!(counts, [("mn", 2)]);
    }

    #[test]
    fn link_types_have_their_colour_and_count() {
        // Legend.tsx:71 `style={{ background: theme.types[type] }}`, line 73 `linkCounts[i] ?? 0`
        let g = fixture();
        let theme = MapTheme::new(true, &g);
        let rows = type_rows(&g, &theme, &[2]);
        assert_eq!(
            rows,
            [
                TypeRow {
                    index: 0,
                    name: "full".to_owned(),
                    color: Some("#2c1810".to_owned()),
                    count: 2,
                },
                TypeRow {
                    index: 1,
                    name: "resembling".to_owned(),
                    color: Some("#6f6152".to_owned()),
                    count: 0,
                },
            ]
        );
    }

    #[cfg(feature = "ssr")]
    fn render_legend(language: palicanon_ui_lib::Language, cookie: &str) -> String {
        use crate::map::ui::test_support::{plain, render};

        let g = fixture();
        let theme = MapTheme::new(false, &g);
        let collections = collection_rows(&g, &theme, None);
        let types = type_rows(&g, &theme, &[1234, 1]);
        let legend = LegendState::parse(Some(cookie));
        plain(&render(language, move || {
            view! {
                <Legend
                    collections=collections
                    types=types
                    legend=legend
                    type_on=vec![true, false]
                    on_color=Callback::new(|_| {})
                    on_hide=Callback::new(|_| {})
                    on_type=Callback::new(|_| {})
                >
                    <p>"sim controls"</p>
                </Legend>
            }
        }))
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_legend_is_labelled_in_each_language() {
        use palicanon_ui_lib::Language;

        // Legend.tsx:30-31, 63 and 72: headings, hint, the types' names
        for (language, words) in [
            (
                Language::En,
                [
                    "Collections",
                    "Click: colour · eye: hide",
                    "Links",
                    "Full parallel",
                    "1,234",
                ],
            ),
            (
                Language::Fr,
                [
                    "Collections",
                    "Clic : colorer · œil : masquer",
                    "Liens",
                    "Parallèle complet",
                    "1\u{202f}234",
                ],
            ),
            (
                Language::Vi,
                [
                    "Bộ kinh",
                    "Nhấp: tô màu · mắt: ẩn",
                    "Liên kết",
                    "Tương đương hoàn toàn",
                    "1.234",
                ],
            ),
        ] {
            let html = render_legend(language, "||1");
            for word in words {
                assert!(html.contains(word), "{language:?} {word}: {html}");
            }
            assert!(html.contains("sim controls"), "the children: {html}");
        }
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn coloured_hidden_and_switched_off_rows_are_marked() {
        use palicanon_ui_lib::Language;

        use crate::map::ui::test_support::{classes, tag_around};

        // Legend.tsx:43-44 coloured: `bg-chip-hover text-heading shadow-[inset_2px_0_0_var(--ps-gold)]`,
        // hidden: `opacity-40`; line 51 the eye's label `Show`/`Hide` + name
        let html = render_legend(Language::En, "mn|dn|1");
        let li_of = |name: &str| {
            let at = html.find(name).expect(name);
            tag_around(&html, &html[html[..at].rfind("<li").expect(name)..at])
        };
        let mn = li_of("Majjhima Nikāya");
        assert!(mn.contains(r#"title="Colour / uncolour""#), "{mn}");
        assert!(
            classes(&mn).contains(&"shadow-[inset_2px_0_0_var(--ps-gold)]"),
            "{mn}"
        );
        assert!(classes(&mn).contains(&"text-heading"), "{mn}");
        assert!(!classes(&mn).contains(&"opacity-40"), "{mn}");
        assert!(classes(&li_of("Dīgha Nikāya")).contains(&"opacity-40"));
        assert!(html.contains(r#"aria-label="Show Dīgha Nikāya""#), "{html}");
        assert!(
            html.contains(r#"aria-label="Hide Majjhima Nikāya""#),
            "{html}"
        );
        assert_eq!(
            html.matches("opacity-40").count(),
            2,
            "dn hidden, resembling off: {html}"
        );
        assert!(html.contains("background:#ff6f8e"), "{html}");
    }
}
