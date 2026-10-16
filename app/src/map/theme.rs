//! The map's colours per theme (`src/map/theme.ts`): the page's background and gold, one
//! colour per collection, one per link type; bright on the dark theme, deep on the light one
//! (white or pastel would vanish). ui-lib's themes are light, dark and system; the React
//! page's sepia is gone (design v4), and drew as light.

use palicanon_ui_lib::Theme;

use crate::map::model::Graph;

/// The collections' colours on the light theme (theme.ts lines 7–9).
const LIGHT_GROUPS: [(&str, &str); 7] = [
    ("dn", "#3d2a1e"),
    ("mn", "#c2185b"),
    ("sn", "#00897b"),
    ("an", "#3949ab"),
    ("kn", "#7b1fa2"),
    ("vinaya", "#d84315"),
    ("abhidhamma", "#558b2f"),
];

/// The link types' colours on the dark theme (theme.ts line 12).
const DARK_TYPES: [(&str, &str); 6] = [
    ("full", "#ffffff"),
    ("sections", "#c7d2fe"),
    ("resembling", "#94a3b8"),
    ("mentions", "#64748b"),
    ("retells", "#64748b"),
    ("similar", "#c9a84c"),
];

/// The link types' colours on the light theme (theme.ts line 13).
const LIGHT_TYPES: [(&str, &str); 6] = [
    ("full", "#2c1810"),
    ("sections", "#5c4a3d"),
    ("resembling", "#6f6152"),
    ("mentions", "#a0978c"),
    ("retells", "#a0978c"),
    ("similar", "#84590a"),
];

/// A collection with no colour of its own (theme.ts line 32's `?? "#8a8a8a"`).
pub const NO_GROUP_COLOR: &str = "#8a8a8a";

/// The page's background (`--ps-page`) and gold (`--ps-gold`) on the dark theme: the React
/// page's fallbacks (theme.ts lines 30–31) and ui-lib's dark tokens.
pub const DARK_PAGE: (&str, &str) = ("#20262e", "#c9a84c");

/// The same on the light theme: ui-lib's light page (`--ps-page`, `style/theme.css` at
/// v0.1.2), so the canvas matches the page around it, and a bright amber for the points:
/// the React page's light gold (`#84590a`) drew them near black on a flat disc (user,
/// 2026-10-16, variant B of two).
pub const LIGHT_PAGE: (&str, &str) = ("#f8f6f0", "#c9952e");

/// The colours the map is drawn with (theme.ts's `MapTheme`).
#[derive(Clone, Debug, PartialEq)]
pub struct MapTheme {
    /// Drawn on a light page.
    pub light: bool,
    /// The page's background, `#rrggbb`.
    pub background: String,
    /// The page's gold, `#rrggbb`: the points before a collection is coloured, the links.
    pub gold: String,
    /// One colour per [`Graph::langs`] entry.
    pub groups: Vec<String>,
    /// The link types' colours, by type name.
    pub types: &'static [(&'static str, &'static str)],
}

/// Whether `theme` draws on a light page; `prefers_light` is the device's
/// `prefers-color-scheme: light`, read for [`Theme::System`] only. theme.ts line 27
/// (`theme !== "dark"`), with ui-lib's system theme resolved.
pub fn is_light(theme: Theme, prefers_light: bool) -> bool {
    match theme {
        Theme::Light => true,
        Theme::Dark => false,
        Theme::System => prefers_light,
    }
}

impl MapTheme {
    /// The map's colours for `graph` on a light page or a dark one (theme.ts lines 26–35):
    /// on the light page a collection takes its deep colour (by `iso`), on the dark one the
    /// graph's own; [`NO_GROUP_COLOR`] when there is none. The background and gold are
    /// [`LIGHT_PAGE`] or [`DARK_PAGE`] until [`MapTheme::with_page`] replaces them.
    pub fn new(light: bool, graph: &Graph) -> MapTheme {
        let (background, gold) = if light { LIGHT_PAGE } else { DARK_PAGE };
        let groups = graph
            .langs
            .iter()
            .enumerate()
            .map(|(i, group)| {
                let own = if light {
                    LIGHT_GROUPS
                        .iter()
                        .find(|(iso, _)| *iso == group.iso)
                        .map(|(_, color)| *color)
                } else {
                    graph
                        .colors
                        .as_ref()
                        .and_then(|colors| colors.get(i))
                        .map(String::as_str)
                };
                own.unwrap_or(NO_GROUP_COLOR).to_owned()
            })
            .collect();
        MapTheme {
            light,
            background: background.to_owned(),
            gold: gold.to_owned(),
            groups,
            types: if light { &LIGHT_TYPES } else { &DARK_TYPES },
        }
    }

    /// The same with the page's background and gold as the browser computes them
    /// (`--ps-page`, `--ps-gold`; theme.ts's `cssVar`): an empty value keeps the default,
    /// as the `||` of theme.ts lines 30–31 did.
    #[must_use]
    pub fn with_page(self, background: &str, gold: &str) -> MapTheme {
        let or = |css: &str, default: String| {
            let css = css.trim();
            if css.is_empty() {
                default
            } else {
                css.to_owned()
            }
        };
        MapTheme {
            background: or(background, self.background),
            gold: or(gold, self.gold),
            ..self
        }
    }

    /// The colour of the link type `name`, when the theme has one.
    pub fn type_color(&self, name: &str) -> Option<&'static str> {
        self.types
            .iter()
            .find(|(type_name, _)| *type_name == name)
            .map(|(_, color)| *color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The light map draws on ui-lib's light page (`--ps-page`, `style/theme.css` at v0.1.2)
    /// so the canvas and the page around it are one colour, its points in a bright amber:
    /// the React page's `#84590a` drew them near black (user, 2026-10-16: B chosen).
    #[test]
    fn the_light_map_is_ui_libs_page_with_a_bright_amber() {
        let theme = MapTheme::new(true, &crate::map::fixture());
        assert_eq!(theme.background, "#f8f6f0");
        assert_eq!(theme.gold, "#c9952e");
    }
    use crate::map::fixture;

    #[test]
    fn only_the_dark_theme_is_dark() {
        // theme.ts:27 `const light = theme !== "dark";`
        assert!(!is_light(Theme::Dark, true));
        assert!(is_light(Theme::Light, false));
        assert!(is_light(Theme::System, true));
        assert!(!is_light(Theme::System, false));
    }

    #[test]
    fn a_collection_is_deep_on_light_and_the_graphs_own_on_dark() {
        // theme.ts:32 `groups: graph.langs.map((l, i) => (light ? LIGHT_GROUPS[l.iso] : graph.colors?.[i]) ?? "#8a8a8a")`
        let g = fixture();
        let light = MapTheme::new(true, &g);
        assert_eq!(light.groups, ["#3d2a1e", "#c2185b", "#7b1fa2"]);
        let dark = MapTheme::new(false, &g);
        assert_eq!(dark.groups, ["#ffffff", "#ff6f8e", "#c792ff"]);
        let mut bare = g.clone();
        bare.colors = None;
        bare.langs[0].iso = "xx".to_owned();
        assert_eq!(MapTheme::new(false, &bare).groups[0], NO_GROUP_COLOR);
        assert_eq!(MapTheme::new(true, &bare).groups[0], NO_GROUP_COLOR);
    }

    #[test]
    fn the_page_colours_default_per_theme_and_follow_the_css() {
        // theme.ts:30-31 `background: cssVar("--ps-page") || "#20262e", gold: cssVar("--ps-gold") || "#c9a84c"`
        let g = fixture();
        let dark = MapTheme::new(false, &g);
        assert_eq!((dark.background.as_str(), dark.gold.as_str()), DARK_PAGE);
        let light = MapTheme::new(true, &g);
        assert_eq!((light.background.as_str(), light.gold.as_str()), LIGHT_PAGE);
        let css = light.clone().with_page("#f8f6f0", "#9e7b22");
        assert_eq!(
            (css.background.as_str(), css.gold.as_str()),
            ("#f8f6f0", "#9e7b22")
        );
        let empty = light.with_page("", " ");
        assert_eq!((empty.background.as_str(), empty.gold.as_str()), LIGHT_PAGE);
    }

    #[test]
    fn link_types_take_the_themes_colour() {
        // theme.ts:33 `types: TYPE_COLORS[light ? "light" : "dark"]`
        let g = fixture();
        assert_eq!(MapTheme::new(false, &g).type_color("full"), Some("#ffffff"));
        assert_eq!(MapTheme::new(true, &g).type_color("full"), Some("#2c1810"));
        assert_eq!(
            MapTheme::new(true, &g).type_color("similar"),
            Some("#84590a")
        );
        assert_eq!(MapTheme::new(true, &g).type_color("unknown"), None);
    }
}
