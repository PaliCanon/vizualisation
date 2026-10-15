//! A chip that opens a small list of choices, each with a hint line under its label
//! (`src/components/Menu.tsx`, from palisearch/web's `FilterBar.tsx`): the views and the
//! texts' language. The chip shows the current choice; arrows move, Enter or Space picks,
//! Escape or a click outside closes (ui-lib's `Popover`).
//!
//! Built here, not taken from ui-lib v0.1.2: its `Menu` has no hint line under a
//! `MenuItem::Choice`, and its labels are `&'static str` (a ui-lib follow-up: a choice with a
//! hint, and a chip trigger showing the current value).

use leptos::prelude::*;
use palicanon_ui_lib::button::{Disclosed, Disclosure, Size, Variant, classes};
use palicanon_ui_lib::{Icon, IconName, Popover};

/// One choice of a [`ChipMenu`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipOption {
    /// Its name, shown in the chip once chosen.
    pub label: String,
    /// A line under it, if any.
    pub hint: Option<String>,
}

/// What a key does in an open or closed menu (Menu.tsx lines 46–57).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuKey {
    /// Open the list.
    Open,
    /// Close it.
    Close,
    /// Move the active row to this index.
    Move(usize),
    /// Choose this index and close.
    Pick(usize),
    /// Nothing.
    Ignore,
}

/// What `key` does with the list `open` or not, row `active` of `len` active: Escape
/// closes; an arrow opens a closed list, else moves, wrapping around; Enter or Space picks
/// the active row of an open list.
pub fn menu_key(key: &str, open: bool, active: usize, len: usize) -> MenuKey {
    match key {
        "Escape" => MenuKey::Close,
        "ArrowDown" | "ArrowUp" if !open => MenuKey::Open,
        _ if len == 0 => MenuKey::Ignore,
        "ArrowDown" => MenuKey::Move((active + 1) % len),
        "ArrowUp" => MenuKey::Move((active + len - 1) % len),
        "Enter" | " " if open => MenuKey::Pick(active.min(len - 1)),
        _ => MenuKey::Ignore,
    }
}

/// The chip and its list. `id` names the list (`aria-controls`), unique on the page.
#[component]
pub fn ChipMenu(
    /// The list's id.
    id: &'static str,
    /// The chip's accessible name ("View", "Texts").
    #[prop(into)]
    name: Signal<String>,
    /// The choices, top to bottom.
    #[prop(into)]
    options: Signal<Vec<ChipOption>>,
    /// The current choice's index.
    #[prop(into)]
    selected: Signal<usize>,
    /// A choice was made: its index.
    on_select: Callback<usize>,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let controls = Disclosed::new(id, open);
    let active = RwSignal::new(selected.get_untracked());
    let pick = Callback::new(move |i: usize| {
        on_select.run(i);
        open.set(false);
    });
    let on_key = move |ev: leptos::ev::KeyboardEvent| {
        let len = options.with_untracked(Vec::len);
        on_menu_key(&ev, open, active, selected.get_untracked(), len, pick);
    };
    let label = move || {
        options.with(|all| {
            all.get(selected.get())
                .map(|o| o.label.clone())
                .unwrap_or_default()
        })
    };
    view! {
        <div class="relative" on:keydown=on_key>
            <button
                type="button"
                class=classes(Variant::Chip, Size::Sm, "")
                aria-haspopup="listbox"
                aria-expanded=move || controls.is_open().to_string()
                aria-controls=id
                aria-label=move || name.get()
                on:click=move |_| controls.toggle()
            >
                {label}
                <span class="rotate-90" aria-hidden="true">
                    <Icon name=IconName::ChevronRight size=12 />
                </span>
            </button>
            <Popover
                controls=controls
                aria_label=name.get_untracked()
                layout="absolute left-0 top-full z-20 mt-2 min-w-[14rem]"
            >
                <ChipOptions id=id options=options selected=selected active=active pick=pick />
            </Popover>
        </div>
    }
}

/// Runs a key pressed in a [`ChipMenu`] (Menu.tsx lines 46–57): `selected` is where an
/// opening list starts, `len` its number of rows.
fn on_menu_key(
    ev: &leptos::ev::KeyboardEvent,
    open: RwSignal<bool>,
    active: RwSignal<usize>,
    selected: usize,
    len: usize,
    pick: Callback<usize>,
) {
    match menu_key(&ev.key(), open.get_untracked(), active.get_untracked(), len) {
        MenuKey::Open => {
            ev.prevent_default();
            active.set(selected);
            open.set(true);
        }
        MenuKey::Close => open.set(false),
        MenuKey::Move(i) => {
            ev.prevent_default();
            active.set(i);
        }
        MenuKey::Pick(i) => {
            ev.prevent_default();
            pick.run(i);
        }
        MenuKey::Ignore => {}
    }
}

/// The list of a [`ChipMenu`]: one row per option, the chosen one checked, the active one
/// (keys, pointer) marked.
#[component]
fn ChipOptions(
    id: &'static str,
    options: Signal<Vec<ChipOption>>,
    selected: Signal<usize>,
    active: RwSignal<usize>,
    pick: Callback<usize>,
) -> impl IntoView {
    let rows = move || {
        options
            .get()
            .into_iter()
            .enumerate()
            .map(|(i, option)| {
                let chosen = move || selected.get() == i;
                let class = move || {
                    let on = if active.get() == i { " bg-chip-hover" } else { "" };
                    format!("flex cursor-pointer items-start gap-2.5 rounded-lg px-2.5 py-2 text-[13px]{on}")
                };
                view! {
                    <li
                        id=format!("{id}-{i}")
                        role="option"
                        aria-selected=move || chosen().to_string()
                        class=class
                        on:pointerenter=move |_| active.set(i)
                        on:click=move |_| pick.run(i)
                    >
                        <span
                            class="mt-0.5 w-3 shrink-0 text-gold"
                            class:invisible=move || !chosen()
                            aria-hidden="true"
                        >
                            <Icon name=IconName::Check size=12 />
                        </span>
                        <span class="min-w-0">
                            <span class=move || {
                                if chosen() { "text-heading" } else { "text-ink" }
                            }>{option.label}</span>
                            {option
                                .hint
                                .map(|hint| {
                                    view! { <span class="block text-xs text-muted">{hint}</span> }
                                })}
                        </span>
                    </li>
                }
            })
            .collect_view()
    };
    view! {
        <ul role="listbox" aria-activedescendant=move || format!("{id}-{}", active.get())>
            {rows}
        </ul>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_open_then_move_around_enter_picks_escape_closes() {
        // Menu.tsx:47 `if (e.key === "Escape") return setOpen(false);`
        assert_eq!(menu_key("Escape", true, 1, 3), MenuKey::Close);
        // Menu.tsx:50 `if (!open) return setOpen(true);`
        assert_eq!(menu_key("ArrowDown", false, 1, 3), MenuKey::Open);
        assert_eq!(menu_key("ArrowUp", false, 1, 3), MenuKey::Open);
        // Menu.tsx:51 `(a + (e.key === "ArrowDown" ? 1 : options.length - 1)) % options.length`
        assert_eq!(menu_key("ArrowDown", true, 1, 3), MenuKey::Move(2));
        assert_eq!(menu_key("ArrowDown", true, 2, 3), MenuKey::Move(0));
        assert_eq!(menu_key("ArrowUp", true, 0, 3), MenuKey::Move(2));
        // Menu.tsx:53 `(e.key === "Enter" || e.key === " ") && open`
        assert_eq!(menu_key("Enter", true, 2, 3), MenuKey::Pick(2));
        assert_eq!(menu_key(" ", true, 0, 3), MenuKey::Pick(0));
        assert_eq!(menu_key("Enter", false, 0, 3), MenuKey::Ignore);
        assert_eq!(menu_key("a", true, 0, 3), MenuKey::Ignore);
        assert_eq!(
            menu_key("ArrowDown", true, 0, 0),
            MenuKey::Ignore,
            "no rows"
        );
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn the_chip_shows_the_choice_and_lists_every_option_with_its_hint() {
        use palicanon_ui_lib::Language;

        use crate::map::ui::test_support::{plain, render, tag_around};

        let options = vec![
            ChipOption {
                label: "Pāli".to_owned(),
                hint: Some("The whole canon".to_owned()),
            },
            ChipOption {
                label: "Français".to_owned(),
                hint: None,
            },
        ];
        let html = plain(&render(Language::En, || {
            view! {
                <ChipMenu
                    id="viz-test-menu"
                    name="Texts".to_owned()
                    options=options
                    selected=1
                    on_select=Callback::new(|_| {})
                />
            }
        }));
        // Menu.tsx:66-71 the chip: `aria-haspopup="listbox"`, `aria-label={name}`, `{label}`
        let chip = tag_around(&html, "aria-haspopup");
        assert!(chip.contains(r#"aria-label="Texts""#), "{chip}");
        assert!(chip.contains(r#"aria-controls="viz-test-menu""#), "{chip}");
        assert!(chip.contains(r#"aria-expanded="false""#), "{chip}");
        let after_chip = &html[html.find(&chip).expect(&html) + chip.len()..];
        assert!(
            after_chip.trim_start().starts_with("Français"),
            "{after_chip}"
        );
        // Menu.tsx:80-98 the list, closed: every option, the selected one marked.
        assert!(tag_around(&html, r#"id="viz-test-menu""#).contains("hidden"));
        assert_eq!(html.matches(r#"role="option""#).count(), 2, "{html}");
        assert!(html.contains("The whole canon"), "{html}");
        let chosen = tag_around(&html, r#"aria-selected="true""#);
        assert!(chosen.contains(r#"id="viz-test-menu-1""#), "{chosen}");
    }
}
