// Map colours per theme. The page colours come from palisearch's CSS variables
// (styles.css, <html data-theme>); the collection and link colours are the map's own:
// bright on the dark theme, deep on the light and sepia ones (white or pastel would vanish).
import type { Theme } from "../settings";
import type { Graph } from "./data";

const LIGHT_GROUPS: Record<string, string> = {
  dn: "#3d2a1e", mn: "#c2185b", sn: "#00897b", an: "#3949ab", kn: "#7b1fa2", vinaya: "#d84315", abhidhamma: "#558b2f",
};

const TYPE_COLORS: Record<"dark" | "light", Record<string, string>> = {
  dark: { full: "#ffffff", sections: "#c7d2fe", resembling: "#94a3b8", mentions: "#64748b", retells: "#64748b", similar: "#c9a84c" },
  light: { full: "#2c1810", sections: "#5c4a3d", resembling: "#6f6152", mentions: "#a0978c", retells: "#a0978c", similar: "#84590a" },
};

export interface MapTheme {
  light: boolean;
  background: string;
  gold: string;
  groups: string[]; // one per graph.langs
  types: Record<string, string>;
}

const cssVar = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

export function mapTheme(theme: Theme, graph: Graph): MapTheme {
  const light = theme !== "dark";
  return {
    light,
    background: cssVar("--ps-page") || "#20262e",
    gold: cssVar("--ps-gold") || "#c9a84c",
    groups: graph.langs.map((l, i) => (light ? LIGHT_GROUPS[l.iso] : graph.colors?.[i]) ?? "#8a8a8a"),
    types: TYPE_COLORS[light ? "light" : "dark"],
  };
}
