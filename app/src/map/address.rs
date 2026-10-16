//! The map's address (design v4): the path is the view and the open text (`/`, `/sim`,
//! `/mn10`, `/sim/mn10`: [`crate::route::Route`]), `?th=` the vectorial views' threshold, and
//! the history entry's state the trail of texts visited (the React page kept the view in
//! `?map=`, the text in the `#fragment` and the trail in `history.state`, `src/App.tsx` lines
//! 143–168). Pure functions, shared by the server and the browser.

use crate::map::model::SimInfo;
use crate::route::{Route, View};

/// The address of the map in `view`, on `uid` when there is one, with `th` (the threshold as
/// the URL writes it, `0.62`) when given and `view` is vectorial: `/sim/mn10?th=0.62`. A
/// text's canonical address is `address(canonical view, Some(uid), None)`: the head's and
/// the sitemap's.
pub(crate) fn address(view: View, uid: Option<&str>, th: Option<&str>) -> String {
    let path = Route::Map {
        view,
        uid: uid.map(str::to_owned),
    }
    .path();
    match th {
        Some(th) if !th.is_empty() && view.is_vectorial() => format!("{path}?th={th}"),
        _ => path,
    }
}

/// The threshold `?th=` gives `view` in the query `query` (the part after `?`): its value
/// percent-decoded; `None` when the query has none, an empty one, one that is not a number,
/// or when `view` is not a vectorial view (the parallels have no threshold). The server's: the
/// browser's router hands the query already decoded.
#[cfg(any(feature = "ssr", test))]
pub(crate) fn th_param(query: Option<&str>, view: View) -> Option<String> {
    if !view.is_vectorial() {
        return None;
    }
    let raw = query?
        .split('&')
        .find_map(|pair| pair.strip_prefix("th="))?;
    let th = percent_decode(raw)?.trim().to_owned();
    th.parse::<f64>().is_ok_and(f64::is_finite).then_some(th)
}

/// `s` with its `%XX` escapes decoded (`+` is not a space here: a number has none); `None`
/// on a broken escape or bytes that are not UTF-8.
#[cfg(any(feature = "ssr", test))]
fn percent_decode(s: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(s.len());
    let mut rest = s.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        if byte == b'%' {
            let hex = std::str::from_utf8(tail.get(..2)?).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            rest = &tail[2..];
        } else {
            bytes.push(byte);
            rest = tail;
        }
    }
    String::from_utf8(bytes).ok()
}

/// The threshold a view opens at: `?th=` when it is a number inside the view's range, else
/// the view's default (App.tsx lines 186–187: `fromUrl >= min && fromUrl <= max ? fromUrl :
/// default`, `Number(null)` being 0).
pub(crate) fn threshold(th: Option<&str>, sim: &SimInfo) -> f64 {
    let asked = match th.map(str::trim) {
        None | Some("") => 0.0,
        Some(th) => th.parse::<f64>().unwrap_or(f64::NAN),
    };
    if asked >= sim.min && asked <= sim.max {
        asked
    } else {
        sim.default
    }
}

/// The trail as a history entry keeps it: the uids, space-separated (a uid has no space).
pub(crate) fn trail_state(trail: &[String]) -> String {
    trail.join(" ")
}

/// The trail a history entry's state holds, `None` when it holds none (an entry the map did
/// not write: a first visit, a link).
pub(crate) fn trail_from_state(state: Option<&str>) -> Option<Vec<String>> {
    let state = state?;
    Some(state.split_whitespace().map(str::to_owned).collect())
}

/// The trail once the address shows `uid` (App.tsx lines 151–158, the `popstate` handler): kept
/// when it already ends there (the map itself moved, or nothing changed); else the history
/// entry's own trail (back, forward) when it ends there; else a new trail of that text alone,
/// or none without a text.
pub(crate) fn trail_at(current: &[String], uid: Option<&str>, state: Option<&str>) -> Vec<String> {
    let Some(uid) = uid else {
        return Vec::new();
    };
    if current.last().is_some_and(|last| last == uid) {
        return current.to_vec();
    }
    match trail_from_state(state) {
        Some(trail) if trail.last().is_some_and(|last| last == uid) => trail,
        _ => vec![uid.to_owned()],
    }
}

/// Where choosing another view goes (App.tsx lines 505–511): the same text in that view, the
/// threshold dropped (each view opens at its own).
pub(crate) fn view_address(view: View, uid: Option<&str>) -> String {
    address(view, uid, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sim() -> SimInfo {
        SimInfo {
            url: "graph-sim.bin".to_owned(),
            min: 0.29,
            max: 0.95,
            default: 0.56,
            range: Some([0.29, 0.95]),
            sha256: None,
        }
    }

    fn uids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn an_address_is_the_view_the_text_and_the_threshold() {
        assert_eq!(address(View::Parallels, None, None), "/");
        assert_eq!(address(View::Parallels, Some("mn10"), None), "/mn10");
        assert_eq!(address(View::Sim, None, Some("0.62")), "/sim?th=0.62");
        assert_eq!(
            address(View::Sim, Some("sn56.11"), Some("0.62")),
            "/sim/sn56.11?th=0.62"
        );
        assert_eq!(address(View::Tsne, Some("mn1"), Some("")), "/tsne/mn1");
        for view in View::ALL {
            let path = address(view, Some("dn1"), None);
            assert_eq!(
                Route::parse(&path),
                Some(Route::Map {
                    view,
                    uid: Some("dn1".to_owned())
                }),
                "{path}"
            );
        }
    }

    #[test]
    fn the_threshold_is_the_urls_inside_the_range_else_the_default() {
        // App.tsx:186 `const th = fromUrl >= g.sim.min && fromUrl <= g.sim.max ? fromUrl : g.sim.default;`
        let sim = sim();
        assert!((threshold(Some("0.62"), &sim) - 0.62).abs() < 1e-12);
        assert!((threshold(Some("0.29"), &sim) - 0.29).abs() < 1e-12);
        assert!((threshold(Some("0.95"), &sim) - 0.95).abs() < 1e-12);
        for th in [
            None,
            Some(""),
            Some("0.2"),
            Some("0.96"),
            Some("abc"),
            Some("NaN"),
        ] {
            assert!(
                (threshold(th, &sim) - 0.56).abs() < 1e-12,
                "{th:?}: {}",
                threshold(th, &sim)
            );
        }
    }

    #[test]
    fn th_is_decoded_and_only_a_vectorial_views() {
        let th = |query: &str, view: View| th_param(Some(query), view);
        assert_eq!(th("th=0.62", View::Sim).as_deref(), Some("0.62"));
        assert_eq!(th("q=mn&th=0%2E62", View::Pca).as_deref(), Some("0.62"));
        assert_eq!(th("th=%200.7", View::Umap).as_deref(), Some("0.7"));
        assert_eq!(th("a=1&th=0.5&b=2", View::Tsne).as_deref(), Some("0.5"));
        for garbage in [
            "th=", "th=abc", "th=%zz", "th=NaN", "th=inf", "th=%FF", "x=0.6", "",
        ] {
            assert_eq!(th(garbage, View::Sim), None, "{garbage}");
        }
        assert_eq!(th_param(None, View::Sim), None);
        assert_eq!(
            th("th=0.62", View::Parallels),
            None,
            "the parallels have none"
        );
        assert_eq!(
            address(View::Parallels, Some("mn10"), Some("0.62")),
            "/mn10",
            "nor does their address"
        );
    }

    #[test]
    fn the_trail_is_kept_in_the_history_entry() {
        let trail = uids(&["dn1", "mn1", "mn10"]);
        assert_eq!(trail_state(&trail), "dn1 mn1 mn10");
        assert_eq!(trail_from_state(Some("dn1 mn1 mn10")), Some(trail));
        assert_eq!(trail_from_state(Some("")), Some(Vec::new()));
        assert_eq!(trail_from_state(None), None);
    }

    #[test]
    fn back_and_forward_walk_the_trail() {
        // App.tsx:153-155 `setTrail((e.state?.trail as string[] | undefined) ?? (uid ? [uid] : []))`
        let at_mn10 = uids(&["dn1", "mn1", "mn10"]);
        assert_eq!(
            trail_at(&at_mn10, Some("mn1"), Some("dn1 mn1")),
            uids(&["dn1", "mn1"]),
            "back one text"
        );
        assert_eq!(
            trail_at(&uids(&["dn1", "mn1"]), Some("mn10"), Some("dn1 mn1 mn10")),
            at_mn10,
            "forward again"
        );
        assert_eq!(
            trail_at(&at_mn10, Some("mn10"), Some("dn1")),
            at_mn10,
            "the map moved there itself: its trail stands"
        );
        assert_eq!(
            trail_at(&at_mn10, Some("snp1.1"), None),
            uids(&["snp1.1"]),
            "an entry without a trail starts one"
        );
        assert_eq!(
            trail_at(&at_mn10, Some("snp1.1"), Some("dn1 mn1")),
            uids(&["snp1.1"]),
            "a state ending elsewhere is not this entry's"
        );
        assert_eq!(trail_at(&at_mn10, None, Some("dn1")), Vec::<String>::new());
    }

    #[test]
    fn another_view_keeps_the_text_and_drops_the_threshold() {
        // App.tsx:507-509 `url.set("map", v); url.delete("th"); …${location.hash}`
        assert_eq!(view_address(View::Sim, Some("mn10")), "/sim/mn10");
        assert_eq!(view_address(View::Parallels, None), "/");
        assert_eq!(view_address(View::Umap, None), "/umap");
    }
}
