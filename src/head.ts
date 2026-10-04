// Copied from palisearch/web/src/head.ts (commit 8d77770). Keep in sync; local changes: the
// canonical origin is viz.palicanon.app.
import { useEffect } from "react";

// Every route's canonical and og:url are built from this origin; there is
// only ever one production host.
const CANONICAL_BASE = "https://viz.palicanon.app";

function ensureMeta(attr: "name" | "property", key: string): HTMLMetaElement {
  let el = document.head.querySelector<HTMLMetaElement>(`meta[${attr}="${key}"]`);
  if (!el) {
    el = document.createElement("meta");
    el.setAttribute(attr, key);
    document.head.appendChild(el);
  }
  return el;
}

function ensureLink(rel: string): HTMLLinkElement {
  let el = document.head.querySelector<HTMLLinkElement>(`link[rel="${rel}"]`);
  if (!el) {
    el = document.createElement("link");
    el.setAttribute("rel", rel);
    document.head.appendChild(el);
  }
  return el;
}

interface HeadOptions {
  title: string;
  /** Plain sentence; also mirrored into og:description when given. */
  description?: string;
  /** Path only (e.g. "/" or "/terms") — joined with CANONICAL_BASE. */
  canonical?: string;
  noindex?: boolean;
}

/**
 * Sets the per-route document title, description, canonical link and
 * robots directive, mirroring title/description/url into the matching
 * og:* tags. This is a single-page app: routes never fully unmount, so
 * nothing is cleaned up here — each route simply overwrites what it cares
 * about the next time it mounts.
 */
export function useHead({ title, description, canonical, noindex = false }: HeadOptions) {
  useEffect(() => {
    document.title = title;
    ensureMeta("property", "og:title").setAttribute("content", title);

    if (description) {
      ensureMeta("name", "description").setAttribute("content", description);
      ensureMeta("property", "og:description").setAttribute("content", description);
    }

    if (canonical) {
      const url = CANONICAL_BASE + canonical;
      ensureLink("canonical").setAttribute("href", url);
      ensureMeta("property", "og:url").setAttribute("content", url);
    }

    if (noindex) {
      ensureMeta("name", "robots").setAttribute("content", "noindex");
    } else {
      document.head.querySelector('meta[name="robots"]')?.remove();
    }
  }, [title, description, canonical, noindex]);
}
