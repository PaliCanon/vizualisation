// Copied from palisearch/web/src/icons.tsx (commit 8d77770). Keep in sync; local changes: only SearchIcon, GearIcon and the footer icons kept; EyeIcon / EyeOffIcon added in the same style.
// Minimal stroke icons, 16px grid, currentColor.
const base = { width: 16, height: 16, viewBox: "0 0 16 16", fill: "none", stroke: "currentColor", strokeWidth: 1.5, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true } as const;

export const SearchIcon = () => (
  <svg {...base}><circle cx="7" cy="7" r="4.5" /><path d="m10.5 10.5 3 3" /></svg>
);
export const EyeIcon = () => (
  <svg {...base}><path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" /><circle cx="8" cy="8" r="2" /></svg>
);
export const EyeOffIcon = () => (
  <svg {...base}><path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" /><path d="m2.5 2.5 11 11" /></svg>
);
export const GearIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
    <circle cx="12" cy="12" r="3" />
  </svg>
);
export const PanelIcon = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M9 4v16" />
  </svg>
);
export const AppStoreIcon = ({ size = 22 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 22 22" aria-hidden="true">
    <defs>
      <linearGradient id="appstore-blue" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#1fb1ff" />
        <stop offset="1" stopColor="#0a6cf5" />
      </linearGradient>
    </defs>
    <rect width="22" height="22" rx="5.5" fill="url(#appstore-blue)" />
    <g stroke="#fff" strokeWidth="1.6" strokeLinecap="round" fill="none">
      <path d="M6.2 15.2 11 6.6l4.8 8.6" />
      <path d="M8 12.4h6" />
      <path d="M4.6 15.2h1.3M16.1 15.2h1.3" />
    </g>
  </svg>
);
export const InstagramIcon = ({ size = 18 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 22 22" aria-hidden="true">
    <defs>
      <radialGradient id="ig-grad" cx="0.3" cy="1.07" r="1.2">
        <stop offset="0" stopColor="#fdf497" />
        <stop offset="0.1" stopColor="#fdf497" />
        <stop offset="0.5" stopColor="#fd5949" />
        <stop offset="0.75" stopColor="#d6249f" />
        <stop offset="1" stopColor="#285AEB" />
      </radialGradient>
    </defs>
    <rect width="22" height="22" rx="5.5" fill="url(#ig-grad)" />
    <g fill="none" stroke="#fff" strokeWidth="1.6" strokeLinecap="round">
      <rect x="5" y="5" width="12" height="12" rx="3.5" />
      <circle cx="11" cy="11" r="2.9" />
      <path d="M14.6 7.4h.01" strokeWidth="2" />
    </g>
  </svg>
);
export const KofiIcon = ({ size = 22 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 22 22" aria-hidden="true">
    <rect width="22" height="22" rx="5.5" fill="#ff5e5b" />
    <g fill="none" stroke="#fff" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5.5 7.5h8.5v4.2a3.3 3.3 0 0 1-3.3 3.3H8.8a3.3 3.3 0 0 1-3.3-3.3V7.5Z" />
      <path d="M14 8.8h1a1.7 1.7 0 0 1 0 3.4h-1" />
    </g>
  </svg>
);
