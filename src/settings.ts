// Copied from palisearch/web/src/settings.ts (commit 8d77770). Keep in sync; local changes: only the theme (no AI settings).
export type Theme = "dark" | "light" | "sepia";

const THEME_KEY = "theme";

export function loadTheme(): Theme {
  const t = document.documentElement.dataset.theme;
  return t === "light" || t === "sepia" ? t : "dark";
}

export function saveTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Private mode: the choice lasts for this visit only.
  }
}
