// Copied from palisearch/web/src/components/FilterBar.tsx (commit 8d77770). Keep in sync; local changes: only the Menu chip (with Option, Chevron); FilterChips left out.
import { useEffect, useId, useRef, useState } from "react";

export interface Option<T extends string> {
  value: T;
  label: string;
  hint?: string;
}

function Chevron() {
  return (
    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="m4 6 4 4 4-4" />
    </svg>
  );
}

// A chip that opens a small menu in the page's own palette. Arrow keys move,
// Enter picks, Escape or a click outside closes.
export function Menu<T extends string>({ label, value, options, onChange, name, align = "left" }: {
  label: string; value: T; options: Option<T>[]; onChange: (v: T) => void; name: string; align?: "left" | "right";
}) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(() => Math.max(0, options.findIndex((o) => o.value === value)));
  const root = useRef<HTMLDivElement>(null);
  const id = useId();

  useEffect(() => {
    if (!open) return;
    const onDown = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onDown);
    return () => document.removeEventListener("pointerdown", onDown);
  }, [open]);

  function pick(i: number) {
    const o = options[i];
    if (o) onChange(o.value);
    setOpen(false);
  }

  function onKey(e: React.KeyboardEvent) {
    if (e.key === "Escape") return setOpen(false);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!open) return setOpen(true);
      setActive((a) => (a + (e.key === "ArrowDown" ? 1 : options.length - 1)) % options.length);
    }
    if ((e.key === "Enter" || e.key === " ") && open) {
      e.preventDefault();
      pick(active);
    }
  }

  return (
    <div ref={root} className="relative" onKeyDown={onKey}>
      <button
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={id}
        aria-label={name}
        onClick={() => setOpen((o) => !o)}
        className={`inline-flex cursor-pointer items-center gap-1.5 whitespace-nowrap rounded-full border px-3 py-1 text-[13px] transition
          ${open ? "border-gold/60 text-heading" : "border-line text-ink hover:border-card-edge hover:text-heading"}`}
      >
        {label}
        <Chevron />
      </button>
      {open && (
        <ul
          id={id}
          role="listbox"
          aria-activedescendant={`${id}-${active}`}
          className={`absolute z-20 mt-2 min-w-[14rem] overflow-hidden rounded-xl border border-card-edge bg-card p-1 shadow-[0_16px_40px_-16px_rgb(0_0_0/0.7)] ${align === "right" ? "right-0" : "left-0"}`}
        >
          {options.map((o, i) => {
            const selected = o.value === value;
            return (
              <li
                key={o.value}
                id={`${id}-${i}`}
                role="option"
                aria-selected={selected}
                onPointerEnter={() => setActive(i)}
                onClick={() => pick(i)}
                className={`flex cursor-pointer items-start gap-2.5 rounded-lg px-2.5 py-2 text-[13px] ${i === active ? "bg-chip-hover" : ""}`}
              >
                <span className={`mt-0.5 w-3 shrink-0 text-gold ${selected ? "" : "invisible"}`} aria-hidden="true">✓</span>
                <span className="min-w-0">
                  <span className={selected ? "text-heading" : "text-ink"}>{o.label}</span>
                  {o.hint && <span className="block text-xs text-muted">{o.hint}</span>}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
