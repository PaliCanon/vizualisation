//! The search box's matching and ranking (`src/App.tsx` lines 359–373, `searchIndex` and
//! `hits`; `normalize` from `src/map/data.ts` line 87).

use unicode_normalization::UnicodeNormalization;

use crate::map::model::{Graph, LangTranslations};

/// The most results the search lists (App.tsx line 372, `.slice(0, 30)`).
pub const MAX_HITS: usize = 30;

/// `s` folded for matching: decomposed (NFD), the combining diacritics (U+0300–U+036F)
/// dropped, lower-cased, without white space; `Saṃyutta Nikāya` → `samyuttanikaya`.
/// data.ts line 87.
pub fn normalize(s: &str) -> String {
    s.nfd()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c) && !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// One text as the search matches it (App.tsx line 359): its uid, its acronym, and its
/// English, Pāli and translated titles, each [`normalize`]d.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The uid, normalized.
    pub uid: String,
    /// The acronym, normalized.
    pub acronym: String,
    /// `"{en} {root} {translated}"`, normalized.
    pub text: String,
}

/// The search index of `graph`, one [`Entry`] per node; `titles` are the translations of
/// the texts' language (none when the texts are in Pāli). App.tsx line 359.
pub fn index(graph: &Graph, titles: Option<&LangTranslations>) -> Vec<Entry> {
    graph
        .nodes
        .iter()
        .map(|node| {
            let translated = titles
                .and_then(|t| t.titles.get(&node.uid))
                .map_or("", String::as_str);
            Entry {
                uid: normalize(&node.uid),
                acronym: normalize(&node.acronym),
                text: normalize(&format!("{} {} {translated}", node.en, node.root)),
            }
        })
        .collect()
}

/// The nodes `query` finds, best first, at most [`MAX_HITS`] (App.tsx lines 360–373): a
/// query of fewer than two characters (normalized) finds nothing; a uid or acronym equal to
/// it scores 3, one that starts with it 2, a title containing it (three characters or
/// more) 1; ties go to the text with more links (`degree`), then to the graph's order: the
/// score and the links compared as a pair (the page summed `score * 1000 + degree`, which a
/// text of 1000 links or more overturned). Texts the texts' language does not have
/// (`available`) are left out.
pub fn hits(
    query: &str,
    index: &[Entry],
    available: Option<&[bool]>,
    degree: impl Fn(usize) -> usize,
) -> Vec<usize> {
    let q = normalize(query);
    let len = q.chars().count();
    if len < 2 {
        return Vec::new();
    }
    let mut scored: Vec<(usize, (u8, usize))> = index
        .iter()
        .enumerate()
        .filter(|(i, _)| available.is_none_or(|on| on.get(*i).copied().unwrap_or(false)))
        .filter_map(|(i, entry)| {
            let score: u8 = if entry.uid == q || entry.acronym == q {
                3
            } else if entry.uid.starts_with(&q) || entry.acronym.starts_with(&q) {
                2
            } else if len >= 3 && entry.text.contains(&q) {
                1
            } else {
                return None;
            };
            Some((i, (score, degree(i))))
        })
        .collect();
    // Stable, as the page's `Array.prototype.sort`: ties keep the graph's order.
    scored.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
    scored.into_iter().take(MAX_HITS).map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::draw::adjacency;
    use crate::map::fixture;

    #[test]
    fn normalize_folds_diacritics_case_and_spaces() {
        // data.ts:87 `s.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase().replace(/\s+/g, '')`
        assert_eq!(normalize("Saṃyutta Nikāya"), "samyuttanikaya");
        assert_eq!(normalize("MN 10"), "mn10");
        assert_eq!(normalize("Satipaṭṭhāna\tSutta"), "satipatthanasutta");
        assert_eq!(normalize("Kinh Niệm Xứ"), "kinhniemxu");
        assert_eq!(normalize(""), "");
    }

    fn fixture_hits(query: &str, available: Option<&[bool]>) -> Vec<usize> {
        let g = fixture();
        let adj = adjacency(g.nodes.len(), &g.edges);
        hits(query, &index(&g, None), available, |i| adj[i].len())
    }

    #[test]
    fn the_index_holds_uid_acronym_and_titles() {
        // App.tsx:359 `text: normalize(`${n[2]} ${n[3]} ${langTitles?.[n[0]] ?? ""}`)`
        let g = fixture();
        let tr = crate::map::model::Translations::from_json(
            r#"{"fr":{"titles":{"mn10":"Les fondements de l'attention"},"by":{},"translators":[]}}"#
                .as_bytes(),
        )
        .expect("translations");
        let idx = index(&g, tr.0.get("fr"));
        assert_eq!(idx[2].uid, "mn10");
        assert_eq!(idx[2].acronym, "mn10");
        assert_eq!(
            idx[2].text,
            "mindfulnessmeditationsatipatthanalesfondementsdel'attention"
        );
        assert_eq!(
            index(&g, None)[0].text,
            "theall-embracingnetofviewsbrahmajala"
        );
    }

    #[test]
    fn exact_beats_prefix_beats_title_then_more_links() {
        // App.tsx:367-370 score 3 equal, 2 prefix, 1 title (3+ characters), `score * 1000 + engine.adj[i]!.length`
        // "mn1": mn1 equal (3), mn10 prefix (2); dn1 and mn2 do not match.
        assert_eq!(fixture_hits("MN 1", None), [1, 2]);
        // "mn": every MN text is a prefix match; mn1 has 3 links, then mn10 and mn2 tie at 1
        // (graph order).
        assert_eq!(fixture_hits("mn", None), [1, 2, 3]);
        // A title: "root" is in mn1's English title only.
        assert_eq!(fixture_hits("root", None), [1]);
        // Pāli without diacritics.
        assert_eq!(fixture_hits("satipatthana", None), [2]);
    }

    #[test]
    fn short_queries_and_missing_texts_find_nothing() {
        // App.tsx:362 `if (nq.length < 2 || !engine) return [];`
        assert_eq!(fixture_hits("m", None), Vec::<usize>::new());
        assert_eq!(fixture_hits(" ", None), Vec::<usize>::new());
        // App.tsx:369 a title match needs three characters: "ur" (Uraga) is too short.
        assert_eq!(fixture_hits("ur", None), Vec::<usize>::new());
        // App.tsx:365 `if (available && !available[i]) return;`
        let available = [true, false, true, true, true];
        assert_eq!(fixture_hits("mn", Some(&available)), [2, 3]);
    }

    #[test]
    fn the_score_outranks_any_number_of_links() {
        // The page summed `score * 1000 + degree`: a prefix with 1001 links passed an exact
        // match. The order is the score, then the links, then the graph's.
        let idx: Vec<Entry> = ["mn1", "mn10", "mn11", "mn12"]
            .iter()
            .map(|uid| Entry {
                uid: (*uid).to_owned(),
                acronym: (*uid).to_owned(),
                text: String::new(),
            })
            .collect();
        let degree = |i: usize| [0, 1001, 5000, usize::MAX][i];
        assert_eq!(hits("mn1", &idx, None, degree), [0, 3, 2, 1]);
        assert_eq!(
            hits("mn1", &idx, None, |_| 7),
            [0, 1, 2, 3],
            "ties: graph order"
        );
    }

    #[test]
    fn at_most_thirty_hits() {
        // App.tsx:372 `.slice(0, 30)`
        let idx: Vec<Entry> = (0..40)
            .map(|k| Entry {
                uid: format!("sn{k}"),
                acronym: format!("sn{k}"),
                text: String::new(),
            })
            .collect();
        let found = hits("sn", &idx, None, |i| i);
        assert_eq!(found.len(), MAX_HITS);
        assert_eq!(found[0], 39, "more links first");
    }
}
