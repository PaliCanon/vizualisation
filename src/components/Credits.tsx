import { useTranslation } from "react-i18next";

// The credits, gold lines — palicanon.app's `Credits` (palisearch/web/src/components/Footer.tsx),
// with the map's sources: SuttaCentral, the translators of the texts shown, the DPD when the
// vectorial space is shown.
export function Credits({ translators, dpd, className = "" }: { translators: string[]; dpd: boolean; className?: string }) {
  const { t, i18n } = useTranslation();
  const names = new Intl.ListFormat(i18n.language, { type: "conjunction" }).format(translators);
  return (
    <p className={`text-[13px] leading-5 text-gold/80 ${className}`}>
      <a href="https://suttacentral.net" target="_blank" rel="noopener" className="hover:text-gold">{t("credits.sources")}</a>
      {translators.length > 0 && (
        <>
          <br />
          {t("credits.translated", { names })}
        </>
      )}
      {dpd && (
        <>
          <br />
          <a href="https://digitalpalidictionary.github.io" target="_blank" rel="noopener" className="hover:text-gold">{t("credits.dpd")}</a>
        </>
      )}
    </p>
  );
}
