//! The legend's state, kept in the `viz_legend` cookie (design v4: colored · hidden · open,
//! host-only) so the server renders the legend as the reader left it. The React page kept
//! it in localStorage (`src/App.tsx` lines 21–30 `loadLegend`, lines 118–125 the panel's
//! `suttasmap.legendOpen`).
//!
//! The value is `<colored>|<hidden>|<open>`: each set a `.`-separated list of collection
//! ids (`dn.mn`), `open` `1` or `0`; for example `mn.kn|vinaya|1`. Every character is a
//! cookie octet (RFC 6265), so the value needs no quoting.

use std::collections::BTreeSet;

use crate::map::model::Graph;

/// The cookie's name.
pub const COOKIE: &str = "viz_legend";

/// Which collections are coloured or hidden, and whether the legend is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Legend {
    /// Collections drawn in their own colour (by `iso`).
    pub colored: BTreeSet<String>,
    /// Collections not drawn (by `iso`).
    pub hidden: BTreeSet<String>,
    /// The legend panel is open.
    pub open: bool,
}

impl Default for Legend {
    /// Nothing coloured, nothing hidden, the panel open: App.tsx line 28 (`{ colored: new
    /// Set(), hidden: new Set() }`) and line 120 (open unless `"0"` was stored).
    fn default() -> Legend {
        Legend {
            colored: BTreeSet::new(),
            hidden: BTreeSet::new(),
            open: true,
        }
    }
}

impl Legend {
    /// The legend a cookie value holds; anything that is not a value [`Legend::to_cookie`]
    /// writes (absent, garbled, a stray character) is the default, as App.tsx line 27's
    /// `catch` returned the empty sets.
    pub fn parse(value: Option<&str>) -> Legend {
        value.and_then(Self::try_parse).unwrap_or_default()
    }

    /// [`Legend::parse`], `None` on anything [`Legend::to_cookie`] does not write.
    fn try_parse(value: &str) -> Option<Legend> {
        let mut fields = value.split('|');
        let (colored, hidden, open) = (fields.next()?, fields.next()?, fields.next()?);
        if fields.next().is_some() {
            return None;
        }
        let open = match open {
            "1" => true,
            "0" => false,
            _ => return None,
        };
        Some(Legend {
            colored: ids(colored)?,
            hidden: ids(hidden)?,
            open,
        })
    }

    /// The cookie's value (sets in sorted order).
    pub fn to_cookie(&self) -> String {
        let join =
            |set: &BTreeSet<String>| set.iter().map(String::as_str).collect::<Vec<_>>().join(".");
        format!(
            "{}|{}|{}",
            join(&self.colored),
            join(&self.hidden),
            if self.open { '1' } else { '0' }
        )
    }

    /// Colour `iso` if it is not, else stop colouring it (App.tsx lines 382–386 `toggle`,
    /// line 465 `onColor`).
    pub fn toggle_colored(&mut self, iso: &str) {
        toggle(&mut self.colored, iso);
    }

    /// Hide `iso` if it is shown, else show it (App.tsx line 466 `onHide`).
    pub fn toggle_hidden(&mut self, iso: &str) {
        toggle(&mut self.hidden, iso);
    }

    /// One flag per `graph.langs` entry: coloured (App.tsx line 265, `setColored`).
    pub fn colored_mask(&self, graph: &Graph) -> Vec<bool> {
        graph
            .langs
            .iter()
            .map(|group| self.colored.contains(&group.iso))
            .collect()
    }

    /// One flag per `graph.langs` entry: shown (App.tsx line 266, `setVisible`).
    pub fn visible_mask(&self, graph: &Graph) -> Vec<bool> {
        graph
            .langs
            .iter()
            .map(|group| !self.hidden.contains(&group.iso))
            .collect()
    }
}

/// A `.`-separated list of collection ids (ASCII letters, digits, `-`, `_`); `None` when an
/// id is empty or has another character. The empty string is the empty set.
fn ids(field: &str) -> Option<BTreeSet<String>> {
    if field.is_empty() {
        return Some(BTreeSet::new());
    }
    field
        .split('.')
        .map(|id| {
            let ok = !id.is_empty()
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
            ok.then(|| id.to_owned())
        })
        .collect()
}

/// Add `iso` to `set`, or remove it when it is there.
fn toggle(set: &mut BTreeSet<String>, iso: &str) {
    if !set.remove(iso) {
        set.insert(iso.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::fixture;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn the_default_colours_nothing_hides_nothing_and_is_open() {
        // App.tsx:28 `return { colored: new Set(), hidden: new Set() };`
        // App.tsx:120 `return localStorage.getItem("suttasmap.legendOpen") !== "0";`
        let legend = Legend::default();
        assert!(legend.colored.is_empty() && legend.hidden.is_empty() && legend.open);
    }

    #[test]
    fn a_cookie_round_trips() {
        for legend in [
            Legend::default(),
            Legend {
                colored: set(&["mn", "kn"]),
                hidden: set(&["vinaya"]),
                open: true,
            },
            Legend {
                colored: set(&[]),
                hidden: set(&["dn", "abhidhamma"]),
                open: false,
            },
        ] {
            let value = legend.to_cookie();
            assert_eq!(Legend::parse(Some(&value)), legend, "{value}");
        }
        assert_eq!(
            Legend {
                colored: set(&["mn", "kn"]),
                hidden: set(&["vinaya"]),
                open: true,
            }
            .to_cookie(),
            "kn.mn|vinaya|1"
        );
        assert_eq!(Legend::default().to_cookie(), "||1");
    }

    #[test]
    fn a_missing_or_garbled_cookie_is_the_default() {
        // App.tsx:27 `} catch {` → the empty sets
        for value in [
            None,
            Some(""),
            Some("garbage"),
            Some("mn|kn"),
            Some("mn|kn|2"),
            Some("mn|kn|1|x"),
            Some("m n|kn|1"),
            Some("mn..kn||1"),
            Some("{\"colored\":[\"mn\"]}"),
        ] {
            assert_eq!(Legend::parse(value), Legend::default(), "{value:?}");
        }
        // An id the views do not know is kept: it only matches nothing (App.tsx:265).
        assert_eq!(Legend::parse(Some("zz||0")).colored, set(&["zz"]));
    }

    #[test]
    fn toggling_adds_then_removes() {
        // App.tsx:382-386 `if (next.has(iso)) next.delete(iso); else next.add(iso);`
        let mut legend = Legend::default();
        legend.toggle_colored("mn");
        legend.toggle_hidden("dn");
        assert_eq!(
            (legend.colored.clone(), legend.hidden.clone()),
            (set(&["mn"]), set(&["dn"]))
        );
        legend.toggle_colored("mn");
        legend.toggle_hidden("dn");
        assert_eq!(legend, Legend::default());
    }

    #[test]
    fn the_masks_follow_the_graphs_collections() {
        // App.tsx:265-266 `graph.langs.map((l) => colored.has(l.iso))`, `!hidden.has(l.iso)`
        let g = fixture();
        let legend = Legend::parse(Some("kn.zz|dn|1"));
        assert_eq!(legend.colored_mask(&g), [false, false, true]);
        assert_eq!(legend.visible_mask(&g), [false, true, true]);
    }
}
