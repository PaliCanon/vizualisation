import { useTranslation } from "react-i18next";
import type { Graph } from "../map/data";
import type { MapTheme } from "../map/theme";
import { Credits } from "./Credits";
import { Section } from "./Section";
import { ReadLink, RefChip, SuttaRef, CollectionDot, type RefData } from "./SuttaRef";
import { TextCard } from "./TextCard";

const fold = (s?: string) => s?.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();

// The open text: breadcrumb of the texts visited, its reference and summary, then its neighbours by
// link type (folding sections of cards), and where all this comes from.
export function TextPanel({ graph, theme, index, neighbours, simOf, refOf, blurbOf, translatorOf, dpd, onPick, onClose, trail, onJump, actions }: {
  graph: Graph;
  theme: MapTheme;
  index: number;
  neighbours: [number, number][];
  simOf: (a: number, b: number) => number | undefined;
  refOf: (i: number) => RefData; // acronym, Pāli title, title in the texts' language
  blurbOf: (i: number) => string | undefined; // SuttaCentral's summary
  translatorOf: (i: number) => string | undefined; // translator of the title shown
  dpd: boolean; // vectorial space: credit the DPD
  onPick: (i: number) => void;
  onClose: () => void;
  trail: { uid: string; label: string }[]; // texts visited, last = this one
  onJump: (k: number) => void;
  actions?: React.ReactNode; // right end of the header bar (the texts' language)
}) {
  const { t } = useTranslation();
  const [uid, , , , coll, group] = graph.nodes[index]!;
  const byType = graph.types.map(() => [] as number[]);
  for (const [j, ty] of neighbours) byType[ty]!.push(j);
  // "Sutta Nipāta · Khuddaka Nikāya"; once when both are the same collection (Saṁyutta / Saṃyutta).
  const names = [graph.collections[coll]?.name, graph.langs[group]?.name].filter((x): x is string => !!x);
  const place = names.filter((n, k) => names.findIndex((m) => fold(m) === fold(n)) === k).join(" · ");
  const translators = [...new Set([index, ...neighbours.map(([j]) => j)].map(translatorOf).filter((x): x is string => !!x))];
  const blurb = blurbOf(index);

  return (
    <div className="relative flex h-full flex-col">
      {/* Header bar, like palicanon.app's side sheets: close chevron, then the texts visited. */}
      <div className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-2.5">
        <button
          type="button"
          onClick={onClose}
          aria-label={t("panel.close")}
          title={t("panel.close")}
          className="grid size-9 shrink-0 cursor-pointer place-items-center rounded-full text-muted transition hover:bg-chip hover:text-heading"
        >
          <svg width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m6 3 5 5-5 5" /></svg>
        </button>
        <ol aria-label={t("panel.trail")} className="flex min-w-0 items-center gap-1 overflow-x-auto">
          {trail.slice(-6).map((s, k, last) => {
            const real = trail.length - last.length + k;
            return (
              <li key={`${real}-${s.uid}`} className="flex items-center gap-1">
                {k > 0 && <span className="text-subtle" aria-hidden="true">›</span>}
                <RefChip label={s.label} current={k === last.length - 1} onClick={() => onJump(real)} />
              </li>
            );
          })}
        </ol>
        {actions && <div className="ml-auto shrink-0">{actions}</div>}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
      <div className="px-4 pt-4">
      <div>
        <SuttaRef {...refOf(index)} variant="heading" />
      </div>
      {blurb && <p className="mt-2 text-sm leading-6 text-muted">{blurb}</p>}
      <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-2 text-xs text-muted">
        <span>{place}</span>
        <ReadLink href={`https://suttacentral.net/${encodeURIComponent(uid)}`}>{t("panel.read")}</ReadLink>
      </div>
      </div>

      <div className="pb-4">
        {byType.map((js, ti) =>
          js.length ? (
            <Section key={ti} id={graph.types[ti]!} title={t(`types.${graph.types[ti]}`)} count={js.length}>
              <ul className="divide-y divide-line px-2">
                {js
                  .sort((a, b) => (simOf(index, b) ?? 0) - (simOf(index, a) ?? 0) || graph.nodes[a]![5] - graph.nodes[b]![5] || graph.nodes[a]![1].localeCompare(graph.nodes[b]![1], undefined, { numeric: true }))
                  .map((j) => (
                    <TextCard key={j} data={refOf(j)} blurb={blurbOf(j)} color={theme.groups[graph.nodes[j]![5]]} score={simOf(index, j)} onClick={() => onPick(j)} />
                  ))}
              </ul>
            </Section>
          ) : null,
        )}
      </div>
      </div>

      <div className="border-t border-line px-4 py-3">
        <Credits translators={translators} dpd={dpd} className="text-[11px]" />
      </div>
    </div>
  );
}

// One text in a compact list (search results): collection colour, reference (and similarity when known).
export function TextRow({ data, color, score, active, onClick, onPointerEnter }: {
  data: RefData;
  color?: string;
  score?: number;
  active?: boolean;
  onClick: () => void;
  onPointerEnter?: () => void;
}) {
  return (
    <li
      onClick={onClick}
      onPointerEnter={onPointerEnter}
      className={`flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-1.5 hover:bg-chip-hover ${active ? "bg-chip-hover" : ""}`}
    >
      <CollectionDot color={color} />
      <SuttaRef {...data} />
      {score !== undefined && <span className="ml-auto shrink-0 pl-2 text-[13px] tabular-nums text-subtle">{score.toFixed(2)}</span>}
    </li>
  );
}
