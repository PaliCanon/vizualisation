//! The map's data as the page reads it: the data files' own types (the `map_formats`
//! crate, task 004) and what `src/App.tsx` and `src/components/TextPanel.tsx` derived from
//! them for one text.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub use map_formats::blurbs::Blurbs;
pub use map_formats::graph::{Collection, Graph, Group, NodeRow, SimInfo};
pub use map_formats::pairs::{Link, Pairs};
pub use map_formats::translations::{LangTranslations, Translations};

/// The most texts the trail keeps (App.tsx line 255, `.slice(-30)`).
pub const MAX_TRAIL: usize = 30;

/// Each uid's index in `graph` (App.tsx line 260, `indexOf`).
pub fn index_of(graph: &Graph) -> HashMap<String, usize> {
    graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.uid.clone(), i))
        .collect()
}

/// A text's reference as cards and rows show it (App.tsx lines 295–301, `refOf`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRef {
    /// `MN 10`.
    pub acronym: String,
    /// The Pāli title.
    pub pali: String,
    /// The title in the glosses' language, else SuttaCentral's English.
    pub title: String,
}

/// The reference of node `i`; `gloss` are the translations of the glosses' language
/// (App.tsx line 298: `translations?.[glossLang]?.titles[n[0]] || n[2]`, so an empty title
/// falls back to English too).
pub fn text_ref(graph: &Graph, i: usize, gloss: Option<&LangTranslations>) -> TextRef {
    let node = &graph.nodes[i];
    let translated = gloss
        .and_then(|tr| tr.titles.get(&node.uid))
        .filter(|title| !title.is_empty());
    TextRef {
        acronym: node.acronym.clone(),
        pali: node.root.clone(),
        title: translated.unwrap_or(&node.en).clone(),
    }
}

/// Who translated `uid` in the glosses' language (App.tsx lines 306–310, `translatorOf`).
pub fn translator_of<'a>(gloss: Option<&'a LangTranslations>, uid: &str) -> Option<&'a str> {
    let tr = gloss?;
    let k = *tr.by.get(uid)?;
    tr.translators
        .get(usize::try_from(k).ok()?)
        .map(String::as_str)
}

/// SuttaCentral's summary of `uid`: in the glosses' language, else in English (App.tsx
/// lines 302–305, `blurbOf`); an empty summary is none, as [`text_ref`]'s empty title (a
/// language without a blurbs file, Vietnamese, gives an empty `lang`).
pub fn blurb_of<'a>(lang: &'a Blurbs, en: &'a Blurbs, uid: &str) -> Option<&'a str> {
    let said = |blurbs: &'a Blurbs| blurbs.get(uid).filter(|blurb| !blurb.trim().is_empty());
    said(lang).or_else(|| said(en))
}

/// Where node `i` is: `"Sutta Nipāta · Khuddaka Nikāya"`, its collection then its group,
/// once when both name the same thing up to diacritics and case (TextPanel.tsx lines 33–35).
pub fn place(graph: &Graph, i: usize) -> String {
    let node = &graph.nodes[i];
    let names = [
        graph
            .collections
            .get(node.coll as usize)
            .map(|c| c.name.as_str()),
        graph
            .langs
            .get(node.group as usize)
            .map(|g| g.name.as_str()),
    ];
    let mut kept: Vec<&str> = Vec::new();
    for name in names.into_iter().flatten().filter(|n| !n.is_empty()) {
        if !kept.iter().any(|k| fold(k) == fold(name)) {
            kept.push(name);
        }
    }
    kept.join(" · ")
}

/// `s` without diacritics, lower-cased (TextPanel.tsx line 9, `fold`).
fn fold(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfd()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The translators the panel credits: of the text, then of its neighbours, each once, in
/// that order (TextPanel.tsx line 36).
pub fn panel_translators(
    graph: &Graph,
    gloss: Option<&LangTranslations>,
    i: usize,
    neighbours: &[usize],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for j in std::iter::once(i).chain(neighbours.iter().copied()) {
        let Some(node) = graph.nodes.get(j) else {
            continue;
        };
        if let Some(name) = translator_of(gloss, &node.uid)
            && !name.is_empty()
            && !out.iter().any(|o| o == name)
        {
            out.push(name.to_owned());
        }
    }
    out
}

/// The trail after choosing `uid` (App.tsx lines 246–256, `choose`): `None` when it is
/// already the open text; following a link of the open text (`linked`) appends it (the
/// last [`MAX_TRAIL`] kept), anything else starts a new trail.
pub fn next_trail(trail: &[String], uid: &str, linked: bool) -> Option<Vec<String>> {
    if trail.last().is_some_and(|last| last == uid) {
        return None;
    }
    if !linked {
        return Some(vec![uid.to_owned()]);
    }
    let mut next: Vec<String> = trail.to_vec();
    next.push(uid.to_owned());
    let drop = next.len().saturating_sub(MAX_TRAIL);
    next.drain(..drop);
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::fixture;

    fn tr() -> Translations {
        Translations::from_json(
            r#"{"fr":{"titles":{"mn1":"La racine de toutes choses","mn10":"","mn2":"Les souillures"},
                       "by":{"mn1":0,"mn2":1,"mn10":0},"translators":["Môhan Wijayaratna","Christian Maës"]}}"#
                .as_bytes(),
        )
        .expect("translations")
    }

    #[test]
    fn index_of_maps_each_uid_to_its_row() {
        // App.tsx:260 `new Map(graph?.nodes.map((n, i) => [n[0], i]) ?? [])`
        let idx = index_of(&fixture());
        assert_eq!(idx.len(), 5);
        assert_eq!(idx["mn10"], 2);
        assert_eq!(idx.get("an1.1"), None);
    }

    #[test]
    fn a_reference_takes_the_translated_title_else_english() {
        // App.tsx:298 `title: translations?.[glossLang]?.titles[n[0]] || n[2]`
        let g = fixture();
        let tr = tr();
        let fr = tr.0.get("fr");
        assert_eq!(
            text_ref(&g, 1, fr),
            TextRef {
                acronym: "MN 1".to_owned(),
                pali: "Mūlapariyāya".to_owned(),
                title: "La racine de toutes choses".to_owned(),
            }
        );
        assert_eq!(
            text_ref(&g, 2, fr).title,
            "Mindfulness Meditation",
            "empty title"
        );
        assert_eq!(text_ref(&g, 0, fr).title, "The All-embracing Net of Views");
        assert_eq!(text_ref(&g, 1, None).title, "The Root of All Things");
    }

    #[test]
    fn the_translator_of_a_credited_text() {
        // App.tsx:308-309 `const k = tr?.by[uid]; return k === undefined ? undefined : tr!.translators[k];`
        let tr = tr();
        assert_eq!(translator_of(tr.0.get("fr"), "mn2"), Some("Christian Maës"));
        assert_eq!(translator_of(tr.0.get("fr"), "dn1"), None);
        assert_eq!(translator_of(None, "mn2"), None);
    }

    #[test]
    fn a_blurb_in_the_language_else_english() {
        // App.tsx:304 `return blurbs.lang[uid] ?? blurbs.en[uid];`
        let fr = Blurbs::from_json(br#"{"mn1":"Le Bouddha"}"#).expect("blurbs");
        let en = Blurbs::from_json(br#"{"mn1":"The Buddha","mn2":"Defilements"}"#).expect("blurbs");
        assert_eq!(blurb_of(&fr, &en, "mn1"), Some("Le Bouddha"));
        assert_eq!(blurb_of(&fr, &en, "mn2"), Some("Defilements"));
        assert_eq!(blurb_of(&fr, &en, "dn1"), None);
    }

    #[test]
    fn an_empty_blurb_falls_back_to_english() {
        // App.tsx:304 `blurbs.lang[uid] ?? blurbs.en[uid]`, with an empty summary taken as
        // none (as `text_ref`'s empty title): SuttaCentral's English says something.
        let fr = Blurbs::from_json(br#"{"mn1":"","mn2":"   "}"#).expect("blurbs");
        let en = Blurbs::from_json(br#"{"mn1":"The Buddha","mn2":"Defilements"}"#).expect("blurbs");
        assert_eq!(blurb_of(&fr, &en, "mn1"), Some("The Buddha"));
        assert_eq!(blurb_of(&fr, &en, "mn2"), Some("Defilements"));
        let none = Blurbs::from_json(br#"{"mn1":""}"#).expect("blurbs");
        assert_eq!(blurb_of(&none, &none, "mn1"), None, "empty in both");
    }

    #[test]
    fn the_place_names_a_collection_and_group_once() {
        // TextPanel.tsx:34-35 `[collections[coll]?.name, langs[group]?.name]`, kept once by `fold`
        let mut g = fixture();
        assert_eq!(place(&g, 4), "Sutta Nipāta · Khuddaka Nikāya");
        assert_eq!(place(&g, 1), "Majjhima Nikāya");
        g.collections[1].name = "Majjhima Nikaya".to_owned();
        assert_eq!(
            place(&g, 1),
            "Majjhima Nikaya",
            "same name up to diacritics"
        );
        g.collections[1].name = String::new();
        assert_eq!(place(&g, 1), "Majjhima Nikāya", "an empty name is left out");
    }

    #[test]
    fn the_panel_credits_each_translator_once() {
        // TextPanel.tsx:36 `[...new Set([index, ...neighbours.map(([j]) => j)].map(translatorOf).filter(...))]`
        let g = fixture();
        let tr = tr();
        assert_eq!(
            panel_translators(&g, tr.0.get("fr"), 1, &[0, 2, 3]),
            ["Môhan Wijayaratna", "Christian Maës"]
        );
        assert_eq!(
            panel_translators(&g, None, 1, &[0, 2, 3]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_link_extends_the_trail_anything_else_starts_one() {
        // App.tsx:252 `if (cur[cur.length - 1] === uid) return;`
        // App.tsx:255 `navigate(linked ? [...cur, uid].slice(-30) : [uid]);`
        let trail: Vec<String> = ["dn1", "mn1"].map(str::to_owned).to_vec();
        assert_eq!(next_trail(&trail, "mn1", true), None);
        assert_eq!(
            next_trail(&trail, "mn10", true),
            Some(["dn1", "mn1", "mn10"].map(str::to_owned).to_vec())
        );
        assert_eq!(
            next_trail(&trail, "snp1.1", false),
            Some(vec!["snp1.1".to_owned()])
        );
        assert_eq!(next_trail(&[], "dn1", false), Some(vec!["dn1".to_owned()]));
        let long: Vec<String> = (0..MAX_TRAIL).map(|k| format!("sn{k}")).collect();
        let next = next_trail(&long, "an1", true).expect("a new text");
        assert_eq!(next.len(), MAX_TRAIL);
        assert_eq!(
            (next[0].as_str(), next[MAX_TRAIL - 1].as_str()),
            ("sn1", "an1")
        );
    }

    #[test]
    fn the_trail_keeps_the_last_thirty_texts() {
        // App.tsx:255 `[...cur, uid].slice(-30)`
        assert_eq!(MAX_TRAIL, 30);
        let mut trail: Vec<String> = Vec::new();
        for k in 0..=MAX_TRAIL + 5 {
            trail = next_trail(&trail, &format!("sn{k}"), true).expect("a new text");
            assert_eq!(trail.len(), (k + 1).min(MAX_TRAIL), "after sn{k}");
        }
        let expected: Vec<String> = (6..=MAX_TRAIL + 5).map(|k| format!("sn{k}")).collect();
        assert_eq!(trail, expected, "the oldest dropped, the order kept");
    }
}
