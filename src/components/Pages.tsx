// Copied from palisearch/web/src/components/Pages.tsx (commit 8d77770). Keep in sync; local changes:
// "Suttas Map" brand and title, the map's own texts (locales/*.json, pages.*), on Acknowledgements
// the translators table built from the data (Translators.tsx), and on Support the websites shown like
// the app (Showcase.tsx).
import { Fragment, useEffect, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useHead } from "../head";
import { InstagramIcon } from "../icons";
import { appStoreUrl, INSTAGRAM_URL } from "./Footer";
import { Translators } from "./Translators";
import { Showcase } from "./Showcase";

// Fill these before going live; the text itself lives in the locale files.
const PUBLISHER = "Thibault Rivière";
const CONTACT = "riviere.thibault@icloud.com";
const UPDATED = "2026-10-03";

export type PageKind = "acknowledgements" | "terms" | "privacy" | "legal" | "support";
export const PAGE_PATHS: Record<PageKind, string> = {
  acknowledgements: "/acknowledgements",
  terms: "/terms",
  privacy: "/privacy",
  legal: "/legal",
  support: "/support",
};
export function pageFromPath(path: string): PageKind | null {
  return (Object.keys(PAGE_PATHS) as PageKind[]).find((k) => PAGE_PATHS[k] === path) ?? null;
}

interface Section {
  h: string;
  p?: string[];
  list?: string[];
  table?: { head: string[]; rows: string[][] };
  /** A prominent button-styled link, e.g. the Ko-fi donation CTA on Support. */
  cta?: { label: string; href: string };
}
interface PageText {
  title: string;
  description: string;
  intro: string[];
  sections: Section[];
  /** Support page only: the app showcase, rendered before the sections. */
  app?: { h: string; tagline: string; cta: string; alts: string[] };
  /** Support page only (local): the websites, shown like the app, after it. */
  web?: { h: string; name: string; tagline: string; cta: string; href: string; alts: string[] }[];
  /** Support page only: the @daily_sutta showcase, rendered before the sections. */
  instagram?: { h: string; tagline: string; cta: string };
}

const H2 = "mt-10 font-serif text-xl font-semibold text-heading";
const P = "mt-3 max-w-[66ch] text-[15px] leading-7 text-ink";
const UL = "mt-3 max-w-[66ch] list-disc space-y-1.5 pl-5 text-[15px] leading-7 text-ink";
const A = "text-gold underline underline-offset-4 hover:text-gold-hover";
const TH = "py-2 pr-4 text-left text-xs font-semibold uppercase tracking-wide text-subtle";
const TD = "py-2 pr-4 align-top text-[14px] leading-6 text-ink break-words";
const BTN = "mt-4 inline-flex items-center gap-2 rounded-lg border border-gold/60 px-4 py-2 text-sm font-semibold text-gold transition hover:bg-gold hover:text-on-gold";

// The App Store screenshots live as plain static assets; order matches the
// alt text array in pages.support.app.alts.
const APP_SHOTS = [
  "/support/app-main.jpg",
  "/support/app-reader.jpg",
  "/support/app-search.jpg",
  "/support/app-concept-graph.jpg",
  "/support/app-notes.jpg",
];

// Screenshots of the websites, in the order of pages.support.web.
const WEB_SHOTS = [
  ["/support/web/palicanon-home.jpg", "/support/web/palicanon-results.jpg"],
  ["/support/web/map-whole.jpg", "/support/web/map-suttacentral.jpg", "/support/web/map-vectorial.jpg", "/support/web/map-french.jpg"],
];

function go(path: string) {
  return (e: React.MouseEvent) => {
    e.preventDefault();
    history.pushState(null, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  };
}

// The locale text carries two marks only: **bold** and [label](href). Internal
// hrefs (starting with /) navigate in place; the rest open a new tab.
function rich(text: string): ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g).map((part, i) => {
    const bold = /^\*\*(.+)\*\*$/.exec(part);
    if (bold) return <strong key={i} className="text-heading">{bold[1]}</strong>;
    const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(part);
    if (link) {
      const [, label, href] = link;
      return href!.startsWith("/") ? (
        <a key={i} href={href} onClick={go(href!)} className={A}>{label}</a>
      ) : (
        <a key={i} href={href} target="_blank" rel="noopener" className={A}>{label}</a>
      );
    }
    return <Fragment key={i}>{part}</Fragment>;
  });
}

export function Page({ kind, onBack }: { kind: PageKind; onBack: () => void }) {
  const { t } = useTranslation();
  const vars = { publisher: PUBLISHER, contact: CONTACT, interpolation: { escapeValue: false } };
  const page = t(`pages.${kind}`, { returnObjects: true, ...vars }) as PageText;
  useHead({ title: `${page.title} — Suttas Map`, description: page.description, canonical: PAGE_PATHS[kind] });
  useEffect(() => {
    window.scrollTo(0, 0);
  }, [kind]);

  return (
    <div className="mx-auto max-w-[64rem] px-6 pb-20">
      <header className="flex items-center gap-3 py-5">
        <button type="button" onClick={onBack} aria-label={t("panel.back")} className="grid size-9 cursor-pointer place-items-center rounded-full text-muted transition hover:bg-chip hover:text-heading">
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M13 8H3M7 4 3 8l4 4" /></svg>
        </button>
        <a href="/" onClick={go("/")} className="flex items-center gap-2 font-serif text-lg font-semibold text-heading">
          <img src="/lotus.png" alt="" width={22} height={22} />
          Suttas Map
        </a>
      </header>
      <nav aria-label={t("footer.nav")} className="mx-auto flex max-w-[44rem] flex-wrap gap-x-8 gap-y-2 border-b border-line pt-4 pb-3 text-[15px]">
        {(Object.keys(PAGE_PATHS) as PageKind[]).map((k) => (
          <a key={k} href={PAGE_PATHS[k]} onClick={go(PAGE_PATHS[k])} className={k === kind ? "text-gold" : "text-muted transition hover:text-heading"}>
            {t(`footer.nav_${k}`)}
          </a>
        ))}
      </nav>
      <main className="mx-auto max-w-[44rem] pt-8">
        <h1 className="font-serif text-3xl font-semibold text-heading">{page.title}</h1>
        {page.intro.map((p, i) => <p key={i} className={P}>{rich(p)}</p>)}
        {page.app && (
          <Showcase
            h={page.app.h}
            name="Tipiṭaka"
            icon="/support/app-icon.jpg"
            tagline={page.app.tagline}
            cta={page.app.cta}
            href={appStoreUrl("PaliWebMapSupport")}
            shots={APP_SHOTS.map((src, i) => ({ src, alt: page.app!.alts[i] ?? "" }))}
            shape="phone"
          />
        )}
        {page.web?.map((w, i) => (
          <Showcase key={w.name} {...w} icon="/lotus.png" shots={WEB_SHOTS[i]!.map((src, k) => ({ src, alt: w.alts[k] ?? "" }))} shape="web" />
        ))}
        {page.instagram && (
          <section>
            <h2 className={H2}>{page.instagram.h}</h2>
            <div className="mt-3 flex items-center gap-3">
              <InstagramIcon size={28} />
              <p className="font-serif text-lg font-semibold text-heading">@daily_sutta</p>
            </div>
            <p className={P}>{page.instagram.tagline}</p>
            <a href={INSTAGRAM_URL} target="_blank" rel="noopener" className={BTN}>{page.instagram.cta} ↗</a>
          </section>
        )}
        {kind === "acknowledgements" && <Translators />}
        {page.sections.map((s, i) => (
          <section key={i}>
            <h2 className={H2}>{s.h}</h2>
            {s.table && (
              <div className="mt-3 overflow-x-auto">
                <table className="w-full max-w-[66ch] border-collapse">
                  <thead>
                    <tr className="border-b border-line">{s.table.head.map((h) => <th key={h} className={TH}>{h}</th>)}</tr>
                  </thead>
                  <tbody className="divide-y divide-line">
                    {s.table.rows.map((r, k) => <tr key={k}>{r.map((c, j) => <td key={j} className={TD}>{rich(c)}</td>)}</tr>)}
                  </tbody>
                </table>
              </div>
            )}
            {s.list && <ul className={UL}>{s.list.map((li, k) => <li key={k}>{rich(li)}</li>)}</ul>}
            {s.p?.map((p, k) => <p key={k} className={P}>{rich(p)}</p>)}
            {s.cta && (
              <a href={s.cta.href} target="_blank" rel="noopener" className={BTN}>
                {s.cta.label} ↗
              </a>
            )}
          </section>
        ))}
        <p className="mt-10 text-xs text-subtle">{t("pages.updated", { date: UPDATED })}</p>
      </main>
    </div>
  );
}

/** Any path that is neither "/" nor one of the legal pages. */
export function NotFoundPage() {
  const { t } = useTranslation();
  useHead({ title: t("head.notfound_title"), noindex: true });
  return (
    <div className="mx-auto flex min-h-dvh max-w-[64rem] flex-col items-center justify-center px-6 text-center">
      <a href="/" onClick={go("/")} className="mb-6 flex items-center gap-2 font-serif text-lg font-semibold text-heading">
        <img src="/lotus.png" alt="" width={22} height={22} />
        {t("brand")}
      </a>
      <p className="mb-2 font-mono text-xs tracking-widest text-subtle">{t("notfound.eyebrow")}</p>
      <h1 className="font-serif text-4xl font-semibold text-heading">{t("notfound.title")}</h1>
      <p className="mt-4 max-w-[42ch] text-[15px] leading-7 text-ink">{t("notfound.body")}</p>
      <a href="/" onClick={go("/")} className="mt-6 text-gold underline underline-offset-4 hover:text-gold-hover">
        {t("notfound.home")}
      </a>
    </div>
  );
}
