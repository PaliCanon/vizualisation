// Copied from palisearch/web/src/components/SettingsPanel.tsx (commit 8d77770). Keep in sync;
// local changes: the AI assistant section is left out (appearance and language only).
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SUPPORTED_LANGS, setLang, type Lang } from "../i18n";
import { loadTheme, saveTheme, type Theme } from "../settings";

const THEMES: { id: Theme; swatch: string }[] = [
  { id: "dark", swatch: "bg-[#20262e]" },
  { id: "light", swatch: "bg-[#fdf6e3]" },
  { id: "sepia", swatch: "bg-[#efdeb4]" },
];
const LANG_NAMES: Record<Lang, string> = { en: "English", fr: "Français", vi: "Tiếng Việt" };

// Appearance and language. Saved as they go.
export function SettingsPanel({ onClose }: { onClose: () => void }) {
  const { t, i18n } = useTranslation();
  const [theme, setTheme] = useState<Theme>(loadTheme);
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    panel.current?.querySelector<HTMLElement>("[role=radio][aria-checked=true]")?.focus();
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-40 flex items-start justify-center overflow-y-auto bg-black/50 p-4 pt-[8vh] backdrop-blur-sm sm:p-6 sm:pt-[10vh]" onPointerDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={panel} role="dialog" aria-modal="true" aria-labelledby="settings-title" className="w-full max-w-[34rem] rounded-2xl border border-card-edge bg-card shadow-[0_30px_80px_-30px_rgb(0_0_0/0.8)]">
        <div className="flex items-center justify-between border-b border-line px-6 py-4">
          <h2 id="settings-title" className="font-serif text-lg font-semibold text-heading">{t("settings.title")}</h2>
          <button type="button" onClick={onClose} aria-label={t("settings.close")} className="grid size-8 cursor-pointer place-items-center rounded-full text-muted transition hover:bg-chip hover:text-heading">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true"><path d="m4 4 8 8M12 4l-8 8" /></svg>
          </button>
        </div>

        <div className="divide-y divide-line">
          <section className="px-6 py-5">
            <h3 className="text-sm font-semibold text-heading">{t("settings.appearance")}</h3>
            <div role="radiogroup" aria-label={t("settings.appearance")} className="mt-3 grid grid-cols-3 gap-2">
              {THEMES.map((th) => (
                <button
                  key={th.id}
                  type="button"
                  role="radio"
                  aria-checked={theme === th.id}
                  onClick={() => { saveTheme(th.id); setTheme(th.id); }}
                  className="flex cursor-pointer items-center gap-2.5 rounded-lg border border-line px-3 py-2 text-sm text-ink transition hover:border-card-edge aria-checked:border-gold aria-checked:text-heading"
                >
                  <span className={`size-4 rounded-full border border-card-edge ${th.swatch}`} aria-hidden="true" />
                  {t(`settings.themes.${th.id}`)}
                </button>
              ))}
            </div>
          </section>

          <section className="px-6 py-5">
            <h3 className="text-sm font-semibold text-heading">{t("settings.language")}</h3>
            <div role="radiogroup" aria-label={t("settings.language")} className="mt-3 flex flex-wrap gap-2">
              {SUPPORTED_LANGS.map((l) => (
                <button
                  key={l}
                  type="button"
                  role="radio"
                  aria-checked={i18n.resolvedLanguage === l}
                  lang={l}
                  onClick={() => setLang(l)}
                  className="cursor-pointer rounded-lg border border-line px-3 py-2 text-sm text-ink transition hover:border-card-edge aria-checked:border-gold aria-checked:text-heading"
                >
                  {LANG_NAMES[l]}
                </button>
              ))}
            </div>
            <p className="mt-2 text-xs text-subtle">{t("settings.language_note")}</p>
          </section>

        </div>
      </div>
    </div>
  );
}
