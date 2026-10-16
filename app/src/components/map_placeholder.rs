//! The map routes' page until the map itself (task 006): the view's name
//! and the text's uid, if any, in a `<main>`, so every map route renders
//! and is told apart.

use leptos::prelude::*;
use leptos_i18n::td_string;

use crate::catalogue::strings;
use crate::head::PageHead;
use crate::route::{Route, View};

/// The placeholder for the map in `view`, centred on `uid` when given.
#[component]
pub(crate) fn MapPlaceholder(view: View, uid: Option<String>) -> impl IntoView {
    let t = strings();
    let path = Route::Map {
        view,
        uid: uid.clone(),
    }
    .path();
    view! {
        <PageHead
            title=td_string!(t.locale(), head_home_title).to_owned()
            description=td_string!(t.locale(), head_home_description).to_owned()
            path=path
        />
        <main class="mx-auto flex w-full max-w-[64rem] flex-1 flex-col items-center justify-center gap-2 px-6 py-20 text-center">
            <h1 class="font-serif text-3xl font-semibold text-heading">
                {move || view.label(t.locale())}
            </h1>
            {uid.map(|uid| view! { <p class="font-mono text-sm text-muted">{uid}</p> })}
        </main>
    }
}
