// Copied from palisearch/web/src/components/LangSwitch.tsx (commit 8d77770). Keep in sync; local changes: none.
import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SUPPORTED_LANGS, setLang, type Lang } from "../i18n";

const FLAGS: Record<Lang, string> = { en: "🇬🇧", fr: "🇫🇷", vi: "🇻🇳" };

const NAMES: Record<Lang, string> = { en: "English", fr: "Français", vi: "Tiếng Việt" };

function currentOf(resolved: string | undefined): Lang {
  return (SUPPORTED_LANGS as readonly string[]).includes(resolved ?? "") ? (resolved as Lang) : "en";
}

/** Compact flag button, next to the settings gear: its flag is the current
 *  language, its menu switches to another one. Same size and hover style as
 *  the gear so it sits next to it without drawing extra attention. */
export function LangSwitch() {
  const { i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const id = useId();
  const current = currentOf(i18n.resolvedLanguage);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onDown);
    return () => document.removeEventListener("pointerdown", onDown);
  }, [open]);

  return (
    <div ref={root} className="relative">
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={id}
        aria-label={NAMES[current]}
        title={NAMES[current]}
        onClick={() => setOpen((o) => !o)}
        className="grid size-8 cursor-pointer place-items-center rounded-lg leading-none transition hover:opacity-90"
      >
        {/* A square flag, rounded like the app icons it sits next to. */}
        {/* The language code, quiet like the gear next to it; flags stay in the menu. */}
        <span className="text-[13px] font-semibold tracking-wide text-muted uppercase">{current}</span>
      </button>
      {open && (
        <ul
          id={id}
          role="listbox"
          className="absolute right-0 z-20 mt-2 min-w-[9.5rem] overflow-hidden rounded-xl border border-card-edge bg-card p-1 shadow-[0_16px_40px_-16px_rgb(0_0_0/0.7)]"
        >
          {SUPPORTED_LANGS.map((l) => {
            const selected = l === current;
            return (
              <li
                key={l}
                role="option"
                aria-selected={selected}
                onClick={() => {
                  setLang(l);
                  setOpen(false);
                }}
                className="flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 text-[13px] hover:bg-chip-hover"
              >
                <span className="w-3 shrink-0 text-gold" aria-hidden="true">{selected ? "✓" : ""}</span>
                <span aria-hidden="true">{FLAGS[l]}</span>
                <span className={selected ? "text-heading" : "text-ink"}>{NAMES[l]}</span>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
