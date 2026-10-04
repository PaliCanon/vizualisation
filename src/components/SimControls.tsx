import { useTranslation } from "react-i18next";

// Vectorial space: similarity threshold and "hide texts with no neighbour".
export function SimControls({ min, max, value, links, alone, hideAlone, onChange, onHideAlone }: {
  min: number;
  max: number;
  value: number;
  links: number;
  alone: number;
  hideAlone: boolean;
  onChange: (v: number) => void;
  onHideAlone: (v: boolean) => void;
}) {
  const { t, i18n } = useTranslation();
  return (
    <div className="mt-3 px-2">
      <label className="block text-[13px] text-ink">
        <span className="flex justify-between">
          {t("sim.threshold")}
          <span className="font-medium tabular-nums text-gold">{value.toFixed(2)}</span>
        </span>
        <input
          type="range"
          min={min}
          max={max}
          step={0.01}
          value={value}
          onChange={(e) => onChange(+e.target.value)}
          className="mt-2 block w-full accent-gold"
        />
      </label>
      <p className="mt-1 text-xs text-subtle">
        {t("sim.stats", { links: links.toLocaleString(i18n.language), alone: alone.toLocaleString(i18n.language) })}
      </p>
      <label className="mt-2 flex cursor-pointer items-center gap-2 text-[13px] text-ink">
        <input type="checkbox" checked={hideAlone} onChange={(e) => onHideAlone(e.target.checked)} className="accent-gold" />
        {t("sim.hide_alone")}
      </label>
    </div>
  );
}
