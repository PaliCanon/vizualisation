// Copied from palisearch/web/src/components/SearchBar.tsx (commit 8d77770). Keep in sync; local changes: no autoFocus (the map is the page).
import { forwardRef, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { SearchIcon } from "../icons";

interface Props {
  value: string;
  large: boolean;
  busy: boolean;
  onChange: (v: string) => void;
  onSubmit: () => void;
}

// One field, one button. "/" and ⌘K focus it from anywhere; the hint lives in
// the footer rather than inside the field.
export const SearchBar = forwardRef<HTMLInputElement, Props>(function SearchBar(
  { value, large, busy, onChange, onSubmit },
  ref,
) {
  const { t } = useTranslation();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName);
      if ((e.key === "k" && (e.metaKey || e.ctrlKey)) || (e.key === "/" && !typing)) {
        e.preventDefault();
        const el = typeof ref === "function" ? null : ref?.current;
        el?.focus();
        el?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [ref]);

  return (
    <form
      role="search"
      onSubmit={(e) => {
        e.preventDefault();
        onSubmit();
      }}
      className="flex items-stretch overflow-hidden rounded-xl border border-card-edge bg-field transition
        focus-within:border-gold focus-within:ring-4 focus-within:ring-glow"
    >
      <span className="flex items-center pl-3.5 text-muted sm:pl-4"><SearchIcon /></span>
      <label htmlFor="q" className="sr-only">{t("search.label")}</label>
      <input
        id="q"
        ref={ref}
        type="search"
        value={value}
        maxLength={200}
        autoComplete="off"
        spellCheck={false}
        placeholder={t("search.placeholder")}
        onChange={(e) => onChange(e.target.value)}
        className={`min-w-0 flex-1 bg-transparent px-3 text-heading outline-none placeholder:text-subtle
          ${large ? "py-3 text-base sm:py-4 sm:text-lg" : "py-2 text-[15px]"}`}
      />
      <button
        type="submit"
        disabled={busy || !value.trim()}
        aria-label={t("search.submit")}
        className={`flex cursor-pointer items-center bg-gold text-sm font-semibold text-on-gold transition hover:bg-gold-hover disabled:cursor-default disabled:opacity-40 ${large ? "px-4 sm:px-6" : "px-3 sm:px-4"}`}
      >
        <span className="hidden sm:inline">{busy ? t("search.searching") : t("search.submit")}</span>
        <span className="sm:hidden" aria-hidden="true">
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><path d="M3 8h9M8.5 4.5 12 8l-3.5 3.5" /></svg>
        </span>
      </button>
    </form>
  );
});
