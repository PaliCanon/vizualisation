// A product on the Support page, in the format palicanon.app uses for the Tipiṭaka app
// (palisearch/web/src/components/Pages.tsx, `page.app`): section title, icon and name, tagline,
// a call to action, then a row of screenshots that scrolls sideways.

const H2 = "mt-10 font-serif text-xl font-semibold text-heading";
const P = "mt-3 max-w-[66ch] text-[15px] leading-7 text-ink";
const BTN = "mt-4 inline-flex items-center gap-2 rounded-lg border border-gold/60 px-4 py-2 text-sm font-semibold text-gold transition hover:bg-gold hover:text-on-gold";

export interface ShowcaseItem {
  h: string;
  name: string;
  icon: string;
  tagline: string;
  cta: string;
  href: string;
  shots: { src: string; alt: string }[];
  // "phone": portrait app screenshots; "web": landscape page screenshots.
  shape: "phone" | "web";
}

export function Showcase({ h, name, icon, tagline, cta, href, shots, shape }: ShowcaseItem) {
  const shot = shape === "phone" ? "w-[150px] sm:w-[180px]" : "w-[300px] border border-line sm:w-[380px]";
  return (
    <section>
      <h2 className={H2}>{h}</h2>
      <div className="mt-3 flex items-center gap-3">
        {shape === "phone" ? (
          <img src={icon} alt="" width={48} height={48} className="rounded-xl" />
        ) : (
          // The websites' lotus, on a tile the size of the app icon.
          <span className="grid size-12 place-items-center rounded-xl border border-card-edge bg-card">
            <img src={icon} alt="" width={28} height={28} />
          </span>
        )}
        <p className="font-serif text-lg font-semibold text-heading">{name}</p>
      </div>
      <p className={P}>{tagline}</p>
      <a href={href} target="_blank" rel="noopener" className={BTN}>{cta} ↗</a>
      <div className="mt-4 flex gap-3 overflow-x-auto pb-2">
        {shots.map(({ src, alt }) => (
          <img key={src} src={src} alt={alt} loading="lazy" className={`h-auto shrink-0 rounded-lg ${shot}`} />
        ))}
      </div>
    </section>
  );
}
