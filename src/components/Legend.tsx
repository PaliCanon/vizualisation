import { useTranslation } from "react-i18next";
import { EyeIcon, EyeOffIcon } from "../icons";
import type { Graph } from "../map/data";
import type { MapTheme } from "../map/theme";

export const sectionTitle = "px-2 text-[11px] font-medium uppercase tracking-wider text-subtle";

// Collections (click: colour, eye: hide) and link types (click: show/hide).
export function Legend({ graph, theme, available, colored, hidden, typeOn, linkCounts, onColor, onHide, onType, children }: {
  graph: Graph;
  theme: MapTheme;
  available: boolean[] | null; // texts shown (translated ones), null: all
  colored: Set<string>;
  hidden: Set<string>;
  typeOn: boolean[];
  linkCounts: number[];
  onColor: (iso: string) => void;
  onHide: (iso: string) => void;
  onType: (t: number) => void;
  children?: React.ReactNode;
}) {
  const { t, i18n } = useTranslation();
  const num = (n: number) => n.toLocaleString(i18n.language);
  const counts = graph.langs.map(() => 0);
  graph.nodes.forEach((n, i) => { if (!available || available[i]) counts[n[5]]!++; });

  return (
    <div className="space-y-5 px-2 py-4">
      <section>
        <h2 className={sectionTitle}>{t("legend.collections")}</h2>
        <p className="mb-2 mt-1 px-2 text-xs text-subtle">{t("legend.hint")}</p>
        <ul>
          {graph.langs.map((l, i) => {
            if (!counts[i]) return null;
            const on = colored.has(l.iso);
            const shown = !hidden.has(l.iso);
            return (
              <li
                key={l.iso}
                onClick={() => onColor(l.iso)}
                title={t("legend.color")}
                className={`flex cursor-pointer select-none items-center gap-2.5 rounded-lg px-2 py-1 text-[13px] transition hover:bg-chip-hover
                  ${on ? "bg-chip-hover text-heading shadow-[inset_2px_0_0_var(--ps-gold)]" : "text-ink"} ${shown ? "" : "opacity-40"}`}
              >
                <span className="size-3 shrink-0 rounded-full" style={{ background: theme.groups[i] }} />
                {l.name}
                <span className="ml-auto tabular-nums text-subtle">{num(counts[i]!)}</span>
                <button
                  type="button"
                  onClick={(e) => { e.stopPropagation(); onHide(l.iso); }}
                  title={shown ? t("legend.hide") : t("legend.show")}
                  aria-label={`${shown ? t("legend.hide") : t("legend.show")} ${l.name}`}
                  className="flex cursor-pointer pl-1 text-subtle transition hover:text-heading"
                >
                  {shown ? <EyeIcon /> : <EyeOffIcon />}
                </button>
              </li>
            );
          })}
        </ul>
      </section>
      <section>
        <h2 className={sectionTitle}>{t("legend.links")}</h2>
        <ul className="mt-1.5">
          {graph.types.map((type, i) => (
            <li
              key={type}
              onClick={() => onType(i)}
              className={`flex cursor-pointer select-none items-center gap-2.5 rounded-lg px-2 py-1 text-[13px] text-ink transition hover:bg-chip-hover ${typeOn[i] ? "" : "opacity-40"}`}
            >
              <span className="h-0.5 w-3.5 shrink-0 rounded-full" style={{ background: theme.types[type] }} />
              {t(`types.${type}`)}
              <span className="ml-auto tabular-nums text-subtle">{num(linkCounts[i] ?? 0)}</span>
            </li>
          ))}
        </ul>
        {children}
      </section>
    </div>
  );
}
