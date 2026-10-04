import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { loadTranslations, type Translations } from "../map/data";

const TH = "py-2 pr-4 text-left text-xs font-semibold uppercase tracking-wide text-subtle";
const TD = "py-2 pr-4 align-top text-[14px] leading-6 text-ink break-words";
const LANGS = [
  { code: "en", name: "English" },
  { code: "fr", name: "Français" },
  { code: "vi", name: "Tiếng Việt" },
];

// Everyone whose work the map shows, one row per language, all named alike and in alphabetical
// order, with how many texts are theirs — read from public/translations.json, so it never drifts
// from the data. Same table style as palicanon.app's pages.
export function Translators() {
  const { t, i18n } = useTranslation();
  const [tr, setTr] = useState<Translations | null>(null);
  useEffect(() => { void loadTranslations().then(setTr); }, []);
  const num = (n: number) => n.toLocaleString(i18n.language);
  // One name per person: credits like "E.B. Cowell, W.H.D. Rouse" list several (a degree such as
  // "D. Litt" stays with its name).
  const sort = (credits: string[]) =>
    [...new Set(credits.flatMap((c) => c.split(/,(?!\s*D\. Litt)\s*/)).map((n) => n.trim()).filter(Boolean))]
      .sort((a, b) => a.localeCompare(b, i18n.language));

  return (
    <section>
      <h2 className="mt-10 font-serif text-xl font-semibold text-heading">{t("credits.table_title")}</h2>
      <p className="mt-3 max-w-[66ch] text-[15px] leading-7 text-ink">{t("credits.table_intro")}</p>
      <div className="mt-3 overflow-x-auto">
        <table className="w-full max-w-[66ch] border-collapse">
          <thead>
            <tr className="border-b border-line">
              <th className={TH}>{t("credits.col_lang")}</th>
              <th className={TH}>{t("credits.col_authors")}</th>
              <th className={TH}>{t("credits.col_texts")}</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-line">
            <tr>
              <td className={TD}>Pāli</td>
              <td className={TD}>{t("credits.pali_edition")}</td>
              <td className={`${TD} tabular-nums`}>{num(7286)}</td>
            </tr>
            {tr && LANGS.map(({ code, name }) => {
              const l = tr[code];
              if (!l) return null;
              return (
                <tr key={code}>
                  <td className={TD}>{name}</td>
                  <td className={TD}>{sort(l.translators).join(", ")}</td>
                  <td className={`${TD} tabular-nums`}>{num(Object.keys(l.titles).length)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}
