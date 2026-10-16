//! What an address naming a text opens (design v4's OPEN node, `open_text(uid, view)`): a
//! uid no view has is a 404; a uid the view has opens its panel with the text highlighted;
//! a uid the view lacks still opens the panel (the text exists), with nothing highlighted.
//! The panel's canonical address is the text's first view, in the views' order (parallels,
//! sim, pca, umap, tsne): each text is in the sitemap once.
//!
//! The React page had no text addresses (the uid was a `#fragment`, `src/App.tsx` lines
//! 153–158) and showed nothing for an unknown one (lines 283–287); this is design v4's rule.
//!
//! The view type is a parameter: the server's text index and the route table plug in
//! (`crate::route::View` and `View::ALL` for the order).

/// What a text's address opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open<V> {
    /// No view has the text: a 404.
    NotFound,
    /// The text's panel.
    Panel {
        /// The text's index in the requested view; `None` when the view lacks it (the panel
        /// opens, nothing is highlighted).
        index_in_view: Option<usize>,
        /// The first view of `views` that has the text: the panel's canonical address.
        canonical_view: V,
    },
}

/// Where a text is: its index in a view, `None` when the view lacks it. The server's text
/// index implements it; so does any `Fn(&str, V) -> Option<usize>`.
pub trait Positions<V> {
    /// The index of `uid` in `view`.
    fn position(&self, uid: &str, view: V) -> Option<usize>;
}

impl<V, F: Fn(&str, V) -> Option<usize>> Positions<V> for F {
    fn position(&self, uid: &str, view: V) -> Option<usize> {
        self(uid, view)
    }
}

/// The canonical view of `uid`: the first of `views` (every view, in canonical order) that
/// has it; `None` when none does. The one rule the server's first render, the head and the
/// sitemap follow.
pub fn canonical_view<V: Copy>(uid: &str, views: &[V], index: &impl Positions<V>) -> Option<V> {
    views
        .iter()
        .copied()
        .find(|v| index.position(uid, *v).is_some())
}

/// What `uid` at `view` opens; `views` are every view, in canonical order.
pub fn open_text<V: Copy>(uid: &str, view: V, views: &[V], index: &impl Positions<V>) -> Open<V> {
    let Some(canonical_view) = canonical_view(uid, views, index) else {
        return Open::NotFound;
    };
    Open::Panel {
        index_in_view: index.position(uid, view),
        canonical_view,
    }
}

/// The text the map highlights for an open panel: none when the texts' language does not
/// have it (App.tsx lines 284–287, `!available || available[i] ? i : -1`).
pub fn highlighted(index_in_view: Option<usize>, available: Option<&[bool]>) -> Option<usize> {
    index_in_view.filter(|&i| available.is_none_or(|on| on.get(i).copied().unwrap_or(false)))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::map::model::{Graph, index_of};
    use crate::map::public_file;
    use crate::route::View;

    /// A view's uid → index, as the server's text index has it.
    type Index = Vec<(View, HashMap<String, usize>)>;

    fn views_of(graphs: &[(View, &Graph)]) -> Index {
        graphs.iter().map(|(v, g)| (*v, index_of(g))).collect()
    }

    /// The index of `uid` in `view`, as a closure over `idx`.
    fn at(idx: &Index) -> impl Fn(&str, View) -> Option<usize> + '_ {
        |uid, view| idx.iter().find(|(v, _)| *v == view)?.1.get(uid).copied()
    }

    fn small() -> (Graph, Graph) {
        let parallels = crate::map::fixture();
        let mut sim = parallels.clone();
        // The vectorial space lacks dn1 and has a text the parallels lack.
        sim.nodes.remove(0);
        sim.nodes[0].uid = "an1.1".to_owned();
        sim.edges.clear();
        (parallels, sim)
    }

    #[test]
    fn an_unknown_uid_is_not_found() {
        let (p, s) = small();
        let idx = views_of(&[(View::Parallels, &p), (View::Sim, &s)]);
        let at = at(&idx);
        assert_eq!(open_text("xx9", View::Sim, &View::ALL, &at), Open::NotFound);
    }

    #[test]
    fn a_text_in_the_view_opens_with_its_index_and_canonical() {
        let (p, s) = small();
        let idx = views_of(&[(View::Parallels, &p), (View::Sim, &s)]);
        let at = at(&idx);
        assert_eq!(
            open_text("mn10", View::Sim, &View::ALL, &at),
            Open::Panel {
                index_in_view: Some(1),
                canonical_view: View::Parallels
            }
        );
        assert_eq!(
            open_text("an1.1", View::Sim, &View::ALL, &at),
            Open::Panel {
                index_in_view: Some(0),
                canonical_view: View::Sim
            }
        );
    }

    #[test]
    fn a_text_missing_from_the_view_opens_with_nothing_highlighted() {
        let (p, s) = small();
        let idx = views_of(&[(View::Parallels, &p), (View::Sim, &s)]);
        let at = at(&idx);
        assert_eq!(
            open_text("dn1", View::Sim, &View::ALL, &at),
            Open::Panel {
                index_in_view: None,
                canonical_view: View::Parallels
            }
        );
        assert_eq!(
            open_text("an1.1", View::Umap, &View::ALL, &at),
            Open::Panel {
                index_in_view: None,
                canonical_view: View::Sim
            }
        );
    }

    #[test]
    fn with_no_view_nothing_opens() {
        let (p, s) = small();
        let idx = views_of(&[(View::Parallels, &p), (View::Sim, &s)]);
        let at = at(&idx);
        for view in View::ALL {
            assert_eq!(
                open_text("mn10", view, &[], &at),
                Open::NotFound,
                "{view:?}"
            );
            assert_eq!(canonical_view("mn10", &[], &at), None, "{view:?}");
        }
        assert_eq!(
            open_text("mn10", View::Sim, &[View::Sim], &at),
            Open::Panel {
                index_in_view: Some(1),
                canonical_view: View::Sim
            },
            "the canonical is among the views given"
        );
    }

    #[test]
    fn only_texts_of_the_language_are_highlighted() {
        // App.tsx:286 `return i !== undefined && (!available || available[i]) ? i : -1;`
        assert_eq!(highlighted(Some(2), None), Some(2));
        assert_eq!(highlighted(Some(2), Some(&[true, true, false])), None);
        assert_eq!(highlighted(Some(1), Some(&[true, true, false])), Some(1));
        assert_eq!(highlighted(None, None), None);
    }

    /// Every uid of every committed view opens its panel at its index, and its canonical
    /// view is the first that has it.
    #[test]
    fn every_uid_of_every_committed_view_opens() {
        let files = [
            (View::Parallels, "graph.json"),
            (View::Sim, "graph-sim.json"),
            (View::Pca, "graph-sim-pca.json"),
            (View::Umap, "graph-sim-umap.json"),
            (View::Tsne, "graph-sim-tsne.json"),
        ];
        let graphs: Vec<(View, Graph)> = files
            .iter()
            .map(|(v, f)| (*v, Graph::from_json(&public_file(f)).expect(f)))
            .collect();
        let refs: Vec<(View, &Graph)> = graphs.iter().map(|(v, g)| (*v, g)).collect();
        let idx = views_of(&refs);
        let at = at(&idx);
        let mut opened = 0usize;
        for (view, g) in &graphs {
            for (i, node) in g.nodes.iter().enumerate() {
                let Open::Panel {
                    index_in_view,
                    canonical_view,
                } = open_text(&node.uid, *view, &View::ALL, &at)
                else {
                    panic!("{} in {view:?} is not found", node.uid);
                };
                assert_eq!(index_in_view, Some(i), "{}", node.uid);
                let first = View::ALL.into_iter().find(|v| at(&node.uid, *v).is_some());
                assert_eq!(Some(canonical_view), first, "{}", node.uid);
                opened += 1;
            }
        }
        assert_eq!(
            opened,
            graphs.iter().map(|(_, g)| g.nodes.len()).sum::<usize>()
        );
    }
}
