//! A text's reference, styled like palicanon.app's (`src/components/SuttaRef.tsx`, from
//! palisearch/web's `ResultList.tsx` titles and `Answer.tsx` citations): the acronym in gold
//! monospace, the Pāli title in serif, the translated title muted. Also the breadcrumb's
//! chip, "Read on SuttaCentral", the collection's dot, a row of the search results
//! (`TextRow`, TextPanel.tsx) and a neighbour's card (`TextCard.tsx`).

use leptos::prelude::*;

use crate::map::model::TextRef;
use crate::map::ui::TextItem;

/// The translated title, unless it is the Pāli one (SuttaRef.tsx line 13, `gloss`).
fn gloss(reference: &TextRef) -> Option<String> {
    (!reference.title.is_empty() && reference.title != reference.pali)
        .then(|| reference.title.clone())
}

/// The Pāli title, if any (SuttaRef.tsx line 6, `pali?`).
fn pali(reference: &TextRef) -> Option<String> {
    (!reference.pali.is_empty()).then(|| reference.pali.clone())
}

/// How a [`SuttaRef`] is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefVariant {
    /// One row of a list, truncated.
    #[default]
    Line,
    /// The open text, at the top of the panel.
    Heading,
}

/// A text's reference: acronym, Pāli title, translated title.
#[component]
pub fn SuttaRef(
    /// What it shows.
    reference: TextRef,
    /// A row (default) or the panel's heading.
    #[prop(optional)]
    variant: RefVariant,
) -> impl IntoView {
    let gloss = gloss(&reference);
    let pali = pali(&reference);
    match variant {
        RefVariant::Heading => view! {
            <div class="min-w-0">
                <p class="font-mono text-xs text-gold">{reference.acronym}</p>
                {pali
                    .map(|pali| {
                        view! {
                            <p
                                class="mt-0.5 font-serif text-xl font-semibold leading-snug text-heading"
                                lang="pi"
                            >
                                {pali}
                            </p>
                        }
                    })}
                {gloss.map(|gloss| view! { <p class="text-[13px] text-muted">{gloss}</p> })}
            </div>
        }
        .into_any(),
        RefVariant::Line => view! {
            <span class="flex min-w-0 items-baseline gap-2">
                <span class="shrink-0 whitespace-nowrap font-mono text-[13px] text-gold">
                    {reference.acronym}
                </span>
                {pali
                    .map(|pali| {
                        view! {
                            <span
                                class="min-w-0 shrink truncate font-serif text-[14px] italic text-heading"
                                lang="pi"
                            >
                                {pali}
                            </span>
                        }
                    })}
                {gloss
                    .map(|gloss| {
                        view! {
                            <span class="min-w-0 shrink-[2] truncate text-[13px] text-muted">
                                {gloss}
                            </span>
                        }
                    })}
            </span>
        }
        .into_any(),
    }
}

/// A reference as a chip (palicanon's citation chip), for the breadcrumb: the current text
/// plain, any other a button running `on_click`.
#[component]
pub fn RefChip(
    /// The acronym shown.
    #[prop(into)]
    label: String,
    /// The text the panel shows (`aria-current="page"`).
    #[prop(optional)]
    current: bool,
    /// What a click does (not called on the current one).
    #[prop(optional, into)]
    on_click: Option<Callback<()>>,
) -> impl IntoView {
    const CHIP: &str = "inline-flex items-center whitespace-nowrap rounded-full border px-2 py-px font-mono text-[11px] text-gold transition";
    if current {
        view! {
            <span class=format!("{CHIP} border-gold/60 bg-chip") aria-current="page">
                {label}
            </span>
        }
        .into_any()
    } else {
        view! {
            <button
                type="button"
                class=format!("{CHIP} cursor-pointer border-line bg-chip hover:border-gold")
                on:click=move |_| {
                    if let Some(on_click) = on_click {
                        on_click.run(());
                    }
                }
            >
                {label}
            </button>
        }
        .into_any()
    }
}

/// "Read on SuttaCentral ↗" (palicanon's read button): a link in a new tab.
#[component]
pub fn ReadLink(
    /// Where it goes.
    #[prop(into)]
    href: String,
    /// The label.
    children: Children,
) -> impl IntoView {
    view! {
        <a
            href=href
            target="_blank"
            rel="noopener"
            class="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border border-line bg-chip px-3 py-1 text-[13px] text-muted transition hover:border-gold/60 hover:text-heading"
        >
            {children()}
            <span aria-hidden="true" class="text-[11px]">
                "↗"
            </span>
        </a>
    }
}

/// A text's collection colour, a dot.
#[component]
pub fn CollectionDot(
    /// `#rrggbb`.
    #[prop(into)]
    color: String,
) -> impl IntoView {
    view! { <span class="size-2.5 shrink-0 self-center rounded-full" style:background=color></span> }
}

/// One text in a compact list (the search results): its collection's dot, its reference,
/// its similarity when known. `active` is the row the keys are on.
#[component]
pub fn TextRow(
    /// The text.
    item: TextItem,
    /// The row the keyboard is on.
    #[prop(into)]
    active: Signal<bool>,
    /// A click on the row.
    on_click: Callback<()>,
    /// The pointer entered the row.
    on_enter: Callback<()>,
) -> impl IntoView {
    let class = move || {
        let on = if active.get() { " bg-chip-hover" } else { "" };
        format!(
            "flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-1.5 hover:bg-chip-hover{on}"
        )
    };
    view! {
        <li
            role="option"
            aria-selected=move || active.get().to_string()
            class=class
            on:click=move |_| on_click.run(())
            on:pointerenter=move |_| on_enter.run(())
        >
            <CollectionDot color=item.color />
            <SuttaRef reference=item.reference />
            {item
                .score
                .map(|score| {
                    view! {
                        <span class="ml-auto shrink-0 pl-2 text-[13px] tabular-nums text-subtle">
                            {format!("{score:.2}")}
                        </span>
                    }
                })}
        </li>
    }
}

/// A text in a list as palicanon.app's chat lists its sources (`TextCard.tsx`): dot,
/// acronym, similarity, Pāli title, translated title, summary; a click opens it.
#[component]
pub fn TextCard(
    /// The text.
    item: TextItem,
    /// A click on the card.
    on_click: Callback<()>,
) -> impl IntoView {
    let gloss = gloss(&item.reference);
    let pali = pali(&item.reference);
    view! {
        <li class="py-1.5">
            <button
                type="button"
                on:click=move |_| on_click.run(())
                class="block w-full cursor-pointer rounded-lg px-2 py-3 text-left text-sm transition hover:bg-chip"
            >
                <span class="flex items-center gap-1.5">
                    <CollectionDot color=item.color />
                    <span class="font-mono text-xs text-gold">{item.reference.acronym}</span>
                    {item
                        .score
                        .map(|score| {
                            view! {
                                <span class="ml-auto text-[11px] tabular-nums text-subtle">
                                    {format!("{score:.2}")}
                                </span>
                            }
                        })}
                </span>
                {pali
                    .map(|pali| {
                        view! {
                            <span class="block font-serif text-[15px] text-heading" lang="pi">
                                {pali}
                            </span>
                        }
                    })}
                {gloss.map(|gloss| view! { <span class="block text-xs text-muted">{gloss}</span> })}
                {item
                    .blurb
                    .map(|blurb| {
                        view! {
                            <span class="mt-1 block text-[13px] leading-5 text-muted">{blurb}</span>
                        }
                    })}
            </button>
        </li>
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use palicanon_ui_lib::Language;

    use super::*;
    use crate::map::ui::test_support::{Fixture, classes, plain, render, tag_around};

    #[test]
    fn a_reference_line_has_acronym_pali_and_gloss() {
        // SuttaRef.tsx:26-29 acronym, Pāli (lang="pi"), gloss
        let f = Fixture::new();
        let item = f.texts(Language::Fr).item(1);
        let html = plain(&render(Language::Fr, || {
            view! { <SuttaRef reference=item.reference /> }
        }));
        assert!(html.contains(">MN 1<"), "{html}");
        assert!(html.contains(r#"lang="pi""#), "{html}");
        assert!(html.contains(">Mūlapariyāya<"), "{html}");
        assert!(html.contains(">La racine de toutes choses<"), "{html}");
        assert!(tag_around(&html, ">MN 1<").contains("font-mono"), "{html}");
    }

    #[test]
    fn a_gloss_equal_to_the_pali_title_is_left_out() {
        // SuttaRef.tsx:13 `const gloss = title && title !== pali ? title : undefined;`
        let reference = TextRef {
            acronym: "Snp 1.1".to_owned(),
            pali: "Uraga".to_owned(),
            title: "Uraga".to_owned(),
        };
        let html = plain(&render(Language::En, || {
            view! { <SuttaRef reference=reference variant=RefVariant::Heading /> }
        }));
        assert_eq!(html.matches("Uraga").count(), 1, "{html}");
        assert!(
            tag_around(&html, ">Uraga<").contains("font-serif text-xl"),
            "{html}"
        );
    }

    #[test]
    fn the_current_chip_is_plain_the_others_buttons() {
        // SuttaRef.tsx:41-46 `current ? <span aria-current="page"> : <button>`
        let html = plain(&render(Language::En, || {
            view! {
                <RefChip label="DN 1" on_click=Callback::new(|()| {}) />
                <RefChip label="MN 1" current=true />
            }
        }));
        assert!(tag_around(&html, ">DN 1<").starts_with("<button"), "{html}");
        let current = tag_around(&html, ">MN 1<");
        assert!(current.starts_with("<span"), "{current}");
        assert!(current.contains(r#"aria-current="page""#), "{current}");
    }

    #[test]
    fn the_read_link_opens_a_new_tab() {
        // SuttaRef.tsx:52-58 `target="_blank" rel="noopener"`, then "↗"
        let html = plain(&render(Language::En, || {
            view! { <ReadLink href="https://suttacentral.net/mn1">"Read"</ReadLink> }
        }));
        let tag = tag_around(&html, "href=");
        assert!(
            tag.contains(r#"href="https://suttacentral.net/mn1""#),
            "{tag}"
        );
        assert!(tag.contains(r#"target="_blank""#) && tag.contains(r#"rel="noopener""#));
        assert!(html.contains("Read") && html.contains('↗'), "{html}");
    }

    #[test]
    fn a_row_is_marked_while_active() {
        // TextPanel.tsx:111 `${active ? "bg-chip-hover" : ""}`
        let f = Fixture::new();
        let item = f.texts(Language::En).item(2);
        let row = |active: bool| {
            let item = item.clone();
            plain(&render(Language::En, move || {
                view! {
                    <TextRow
                        item=item
                        active=active
                        on_click=Callback::new(|()| {})
                        on_enter=Callback::new(|()| {})
                    />
                }
            }))
        };
        let on = row(true);
        let li = tag_around(&on, "<li");
        assert!(classes(&li).contains(&"bg-chip-hover"), "{li}");
        assert!(
            on.contains("background:#ff6f8e"),
            "MN 10's collection: {on}"
        );
        assert!(on.contains(">MN 10<"), "{on}");
        let off = row(false);
        assert!(!classes(&tag_around(&off, "<li")).contains(&"bg-chip-hover"));
    }

    #[test]
    fn a_card_shows_the_summary_and_the_similarity() {
        // TextCard.tsx:19 `score.toFixed(2)`, line 23 the blurb
        let f = Fixture::new();
        let mut item = f.texts(Language::En).item(2);
        item.score = Some(0.834);
        let html = plain(&render(Language::En, || {
            view! { <TextCard item=item on_click=Callback::new(|()| {}) /> }
        }));
        assert!(html.contains(">0.83<"), "{html}");
        assert!(html.contains(">The four kinds of mindfulness.<"), "{html}");
        assert!(html.contains(">Satipaṭṭhāna<"), "{html}");
        assert!(tag_around(&html, "<button").contains(r#"type="button""#));
    }
}
