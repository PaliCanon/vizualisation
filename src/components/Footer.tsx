// Copied from palisearch/web/src/components/Footer.tsx (commit 8d77770). Keep in sync; local changes:
// no EnvBadge, no Credits (the map's are in Credits.tsx), nav links lead to the map's own pages,
// optional `leading` slot on the left (the map puts the icons there, under its legend column).
import { useTranslation } from "react-i18next";
import { AppStoreIcon, InstagramIcon, KofiIcon } from "../icons";

// Campaign link (App Store Connect analytics, campaign "PaliWeb").
/** App Store link tagged with an App Store Connect campaign (`ct`), one per
 *  place on the site, so Analytics → Campaigns shows where installs come
 *  from: PaliWebHome, PaliWebFooter, PaliWebChat, PaliWebSupport,
 *  PaliWebPromo — and PaliWebQR, baked into the promo's QR code. */
export function appStoreUrl(campaign: string): string {
  return `https://apps.apple.com/app/apple-store/id6760995639?pt=128678815&ct=${campaign}&mt=8`;
}
export const APP_STORE_URL = appStoreUrl("PaliWeb");
export const INSTAGRAM_URL = "https://www.instagram.com/daily_sutta";
export const KOFI_URL = "https://ko-fi.com/tipitaka";

const LINKS = [
  { href: "", key: "app", Icon: AppStoreIcon },
  { href: INSTAGRAM_URL, key: "instagram", Icon: InstagramIcon },
  { href: KOFI_URL, key: "kofi", Icon: KofiIcon },
] as const;

/** The three outbound icons, reused wherever the page has a quiet corner. */
export function SocialLinks({ size = 24, gap = "gap-5", vertical = false, campaign = "PaliWeb" }: { size?: number; gap?: string; vertical?: boolean; campaign?: string }) {
  const { t } = useTranslation();
  return (
    <ul className={`flex items-center ${vertical ? "flex-col" : ""} ${gap}`}>
      {LINKS.map(({ href, key, Icon }) => (
        <li key={key}>
          <a
            href={key === "app" ? appStoreUrl(campaign) : href}
            target="_blank"
            rel="noopener"
            aria-label={t(`footer.${key}`)}
            title={t(`footer.${key}`)}
            className="block rounded-md transition hover:-translate-y-0.5 hover:opacity-90"
          >
            <Icon size={size} />
          </a>
        </li>
      ))}
    </ul>
  );
}

function go(path: string) {
  return (e: React.MouseEvent) => {
    e.preventDefault();
    history.pushState(null, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  };
}

const NAV = [
  { path: "/acknowledgements", key: "acknowledgements" },
  { path: "/terms", key: "terms" },
  { path: "/privacy", key: "privacy" },
  { path: "/legal", key: "legal" },
  { path: "/support", key: "support" },
] as const;

// A row of plain links, evenly spread, the icons beneath. Each link is a
// section of its own on the About page; nothing is spelled out down here.
/** `social` off where the icons are already shown. */
/** `leading`: shown at the left edge, in a 16rem slot that lines up with the map's legend column. */
export function Footer({ social = true, leading }: { social?: boolean; leading?: React.ReactNode }) {
  const { t } = useTranslation();
  return (
    <footer className={`border-t border-line ${leading ? "flex flex-col items-center md:grid md:grid-cols-[16rem_minmax(0,1fr)_16rem]" : ""}`}>
      {/* With `leading`, an empty 16rem column on the right keeps the links centred on the page. */}
      {leading && <div className="flex justify-center pt-6 md:py-6">{leading}</div>}
      <div className="mx-auto flex w-full min-w-0 max-w-[64rem] flex-col items-center gap-5 px-6 py-6">
        {social && <SocialLinks size={22} gap="gap-4" campaign="PaliWebFooter" />}
        <nav aria-label={t("footer.nav")} className="flex flex-wrap justify-center gap-x-3 gap-y-2 sm:gap-x-10">
          {NAV.map(({ path, key }) => (
            <a key={key} href={path} onClick={go(path)} className="text-[13px] text-muted transition hover:text-gold sm:text-[15px]">
              {t(`footer.nav_${key}`)}
            </a>
          ))}
        </nav>
      </div>
    </footer>
  );
}
