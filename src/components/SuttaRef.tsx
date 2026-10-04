// A text's reference, styled like palicanon.app's (palisearch/web/src/components/ResultList.tsx
// result titles and Answer.tsx citations): acronym in gold monospace, Pāli title in serif,
// translated title muted.

export interface RefData {
  acronym: string; // "MN 10"
  pali?: string; // "Satipaṭṭhānasutta"
  title?: string; // translated title, in the texts' language ("Mindfulness Meditation")
}

// "line": one row in a list, truncated. "heading": the open text, at the top of the panel.
export function SuttaRef({ acronym, pali, title, variant = "line" }: RefData & { variant?: "line" | "heading" }) {
  const gloss = title && title !== pali ? title : undefined;
  if (variant === "heading") {
    return (
      <div className="min-w-0">
        <p className="font-mono text-xs text-gold">{acronym}</p>
        {pali && <p className="mt-0.5 font-serif text-xl font-semibold leading-snug text-heading" lang="pi">{pali}</p>}
        {gloss && <p className="text-[13px] text-muted">{gloss}</p>}
      </div>
    );
  }
  return (
    <span className="flex min-w-0 items-baseline gap-2">
      <span className="shrink-0 whitespace-nowrap font-mono text-[13px] text-gold">{acronym}</span>
      {pali && <span className="min-w-0 shrink truncate font-serif text-[14px] italic text-heading" lang="pi">{pali}</span>}
      {gloss && <span className="min-w-0 shrink-[2] truncate text-[13px] text-muted">{gloss}</span>}
    </span>
  );
}

// A reference as a chip (palicanon's citation chip): for the breadcrumb.
export function RefChip({ label, current, onClick }: { label: string; current?: boolean; onClick?: () => void }) {
  const cls = "inline-flex items-center whitespace-nowrap rounded-full border px-2 py-px font-mono text-[11px] text-gold transition";
  return current ? (
    <span className={`${cls} border-gold/60 bg-chip`} aria-current="page">{label}</span>
  ) : (
    <button type="button" onClick={onClick} className={`${cls} cursor-pointer border-line bg-chip hover:border-gold`}>
      {label}
    </button>
  );
}

// "Read on SuttaCentral ↗" (palicanon's read button).
export function ReadLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener"
      className="inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border border-line bg-chip px-3 py-1 text-[13px] text-muted transition hover:border-gold/60 hover:text-heading"
    >
      {children}
      <span aria-hidden="true" className="text-[11px]">↗</span>
    </a>
  );
}

// The collection colour of a text.
export function CollectionDot({ color }: { color?: string }) {
  return <span className="size-2.5 shrink-0 self-center rounded-full" style={{ background: color }} />;
}
