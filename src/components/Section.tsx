import { useState } from "react";

function Chevron({ open }: { open: boolean }) {
  return (
    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"
      className={`transition-transform ${open ? "" : "-rotate-90"}`}>
      <path d="m4 6 4 4 4-4" />
    </svg>
  );
}

// A titled section that folds, closed until opened. Open/closed is remembered per `id`
// (localStorage), only once the reader has toggled it (v2: the earlier key stored "open" for every
// section on first display, which would keep them all open).
export function Section({ id, title, count, children }: { id: string; title: string; count?: number; children: React.ReactNode }) {
  const key = `suttasmap.section.v2.${id}`;
  const [open, setOpen] = useState(() => {
    try { return localStorage.getItem(key) === "1"; } catch { return false; }
  });
  const toggle = () => setOpen((o) => {
    try { localStorage.setItem(key, o ? "0" : "1"); } catch { /* private mode */ }
    return !o;
  });
  return (
    <section>
      <button
        type="button"
        onClick={toggle}
        aria-expanded={open}
        className="flex w-full cursor-pointer items-center gap-1.5 px-4 pb-1 pt-4 text-[11px] font-medium uppercase tracking-wider text-subtle transition hover:text-heading"
      >
        <Chevron open={open} />
        {title}
        {count !== undefined && <span className="tabular-nums">· {count}</span>}
      </button>
      {open && children}
    </section>
  );
}
