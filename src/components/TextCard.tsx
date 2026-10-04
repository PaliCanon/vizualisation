import { CollectionDot, type RefData } from "./SuttaRef";

// A text in a list, as palicanon.app's chat lists the suttas given to the model
// (palisearch/web/src/components/ChatView.tsx, `sources`): acronym in gold monospace, Pāli title in
// serif, translated title, then SuttaCentral's summary. The collection dot is the map's own.
export function TextCard({ data, blurb, color, score, onClick }: {
  data: RefData;
  blurb?: string;
  color?: string;
  score?: number;
  onClick: () => void;
}) {
  const gloss = data.title && data.title !== data.pali ? data.title : undefined;
  return (
    <li className="py-1.5">
      <button type="button" onClick={onClick} className="block w-full cursor-pointer rounded-lg px-2 py-3 text-left text-sm transition hover:bg-chip">
        <span className="flex items-center gap-1.5">
          <CollectionDot color={color} />
          <span className="font-mono text-xs text-gold">{data.acronym}</span>
          {score !== undefined && <span className="ml-auto text-[11px] tabular-nums text-subtle">{score.toFixed(2)}</span>}
        </span>
        {data.pali && <span className="block font-serif text-[15px] text-heading" lang="pi">{data.pali}</span>}
        {gloss && <span className="block text-xs text-muted">{gloss}</span>}
        {blurb && <span className="mt-1 block text-[13px] leading-5 text-muted">{blurb}</span>}
      </button>
    </li>
  );
}
