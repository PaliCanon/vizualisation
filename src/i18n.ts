// Copied from palisearch/web/src/i18n.ts (commit 8d77770). Keep in sync; local changes: none (the locales are this app's own).
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import fr from "./locales/fr.json";
import vi from "./locales/vi.json";

// One JSON per locale, same keys. The interface is translated, and searches go
// to the UI language's index (English when that one is not deployed; the
// model is multilingual, so search() falls back there and re-searches). Sutta
// titles and the AI's quotations use the app's French or Vietnamese
// translation when the API has one for that sutta. A new language = a JSON here, an entry in
// SUPPORTED_LANGS, its name in SettingsPanel and its line in ai.ts.
export const SUPPORTED_LANGS = ["en", "fr", "vi"] as const;
export type Lang = (typeof SUPPORTED_LANGS)[number];

const LANG_KEY = "palisearch.lang";

function isLang(v: unknown): v is Lang {
  return typeof v === "string" && (SUPPORTED_LANGS as readonly string[]).includes(v);
}

function stored(): string | null {
  try {
    return localStorage.getItem(LANG_KEY);
  } catch {
    return null;
  }
}

/**
 * `?lang=vi` (or any supported code) in the URL picks and saves that language —
 * handy for sharing a link in a given language and for headless screenshots.
 * Otherwise: the saved choice, else the browser's language, else English.
 */
function initialLang(): Lang {
  const fromUrl = new URLSearchParams(location.search).get("lang");
  if (isLang(fromUrl)) {
    saveLang(fromUrl);
    return fromUrl;
  }
  const saved = stored();
  if (isLang(saved)) return saved;
  const nav = (navigator.language ?? "").toLowerCase().slice(0, 2);
  return isLang(nav) ? nav : "en";
}

export function saveLang(lang: Lang) {
  try {
    localStorage.setItem(LANG_KEY, lang);
  } catch {
    // Private mode: the choice lasts for this visit only.
  }
}

/** Switches the UI language and persists the choice — the one place that
 *  does both, used by the settings panel and the quick language switcher. */
export function setLang(lang: Lang) {
  saveLang(lang);
  void i18n.changeLanguage(lang);
}

/** The current UI language, narrowed to a supported one. */
export function currentLang(): Lang {
  return isLang(i18n.resolvedLanguage) ? i18n.resolvedLanguage : "en";
}

i18n.on("languageChanged", (lng) => {
  document.documentElement.lang = lng;
});

void i18n.use(initReactI18next).init({
  resources: { en: { translation: en }, fr: { translation: fr }, vi: { translation: vi } },
  lng: initialLang(),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});
document.documentElement.lang = i18n.language;

export default i18n;
