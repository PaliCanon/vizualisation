"""Ports the React site's strings (`src/locales/<lang>.json`, i18next) to the Leptos app (task 005),
stdlib only, plus `rustfmt` on the PATH:

    python3 scripts/port_strings.py [--out DIR]

-> app/locales/<lang>.json        the `leptos_i18n` catalogue: every key but `pages.*`, flattened
                                  to snake_case (`search.placeholder` -> `search_placeholder`,
                                  `noGpu.title` -> `no_gpu_title`), values verbatim, sorted.
                                  `pages.updated` -> `pages_updated` (its `{{date}}` stays).
-> app/src/pages/content_<lang>.rs  `pages.*` as typed Rust const data (`leptos_i18n` reads a JSON
                                  array as a plural range, so a page's lists cannot live in the
                                  catalogue); `{{publisher}}` and `{{contact}}` resolved with the
                                  values `src/components/Pages.tsx` passes; the public hosts
                                  rewritten by HOSTS.
-> app/src/pages/types.rs         the types those modules fill.

i18next plurals (`tooltip.links_one` / `_other`) become two plain keys, the word alone
(`tooltip_link`, `tooltip_links`), as search-front's `results_one` / `results_label`: the count is
put before it in Rust. `leptos_i18n` 0.6 deprecates range keys (a warning, which CI denies) and its
suffix plurals need the `plurals` feature and ICU data, for one tooltip.

Hosts: the React catalogues link to the map at `viz.palicanon.app`, the React site's host until
task 008; the Leptos app is served at `map.palicanon.app` (user, 2026-10-16). HOSTS maps each old
host to the new one in every page string, and leaves the React catalogues as they are.

Dropped: `settings.themes.sepia` (design v4 has no sepia theme). The key map, old -> new, and every
`t()` call site are in app/STRINGS.md.
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANGS = ("en", "fr", "vi")
LANG_NAMES = {"en": "English", "fr": "French", "vi": "Vietnamese"}
PAGES_TSX = ROOT / "src" / "components" / "Pages.tsx"

DROPPED = {"settings.themes.sepia"}
# Keys the Leptos app adds, which the React site never had: the shared rail's entries,
# worded as learn-pali's (app/locales: nav_search, nav_learn) plus this service's own.
ADDED = {
    "en": {"nav_learn": "Learn", "nav_map": "Map", "nav_search": "Search"},
    "fr": {"nav_learn": "Apprendre", "nav_map": "Carte", "nav_search": "Recherche"},
    "vi": {"nav_learn": "Học", "nav_map": "Bản đồ", "nav_search": "Tìm kiếm"},
}
# i18next plural forms -> the counts CLDR gives `one` in each language (the rest is `_other`).
PLURAL_SUFFIXES = ("_one", "_other")
# Each plural's singular key (the plural keeps the stem's name).
SINGULAR = {"tooltip_links": "tooltip_link"}
# The React site's public hosts -> the Leptos app's (user, 2026-10-16), rewritten in page strings.
HOSTS = {"viz.palicanon.app": "map.palicanon.app", "viz.dev.palicanon.app": "map.dev.palicanon.app"}
# A host on its own: not the tail of another (`dev.viz.palicanon.app`, the Pages dev host).
HOST_RE = re.compile(r"(?<![\w.-])(" + "|".join(re.escape(h) for h in HOSTS) + r")(?![\w-]|\.\w)")

PAGE_ORDER = ("acknowledgements", "terms", "privacy", "legal", "support")
PAGE_KEYS = {"title", "description", "intro", "sections", "app", "instagram", "web"}
SECTION_KEYS = {"h", "p", "list", "table", "cta"}

TYPES_RS = '''\
//! The footer pages' shapes, filled by `content_<lang>.rs`: copied from
//! search-front's `app/src/pages/mod.rs` and adapted to the React site's
//! `pages.*` (rendered by its `src/components/Pages.tsx`) — a Legal page,
//! and on Support the websites (`pages.support.web`) instead of
//! search-front's search showcase. Written by `scripts/port_strings.py`.

/// Every footer page in one language. The "Last updated" line closing
/// each page is the catalogue's `pages_updated`, not here.
#[derive(Debug)]
pub(crate) struct Pages {
    /// Acknowledgements.
    pub acknowledgements: PageText,
    /// Terms of use.
    pub terms: PageText,
    /// Privacy.
    pub privacy: PageText,
    /// Legal notice.
    pub legal: PageText,
    /// Support.
    pub support: PageText,
}

/// One page's text, in the order the page shows it. Paragraphs, list
/// items and table cells are rich text: `**bold**` and `[label](href)`,
/// an `href` starting with `/` being a page of this site.
#[derive(Debug)]
pub(crate) struct PageText {
    /// The `<h1>`, and the document title before " — Suttas Map".
    pub title: &'static str,
    /// The page's meta description.
    pub description: &'static str,
    /// The paragraphs under the title.
    pub intro: &'static [&'static str],
    /// Support only: the Tipiṭaka app's showcase, after the intro.
    pub app: Option<AppBlock>,
    /// Support only: the websites' showcases, after the app's.
    pub sites: &'static [SiteBlock],
    /// Support only: the `@daily_sutta` showcase, after the websites'.
    pub instagram: Option<InstagramBlock>,
    /// The page's sections, in order, after the showcases (on
    /// Acknowledgements, after the translators table).
    pub sections: &'static [Section],
}

/// The Tipiṭaka app's showcase (Support).
#[derive(Debug)]
pub(crate) struct AppBlock {
    /// Its heading.
    pub h: &'static str,
    /// One line on the app.
    pub tagline: &'static str,
    /// The App Store button's label.
    pub cta: &'static str,
    /// The screenshots' alt texts, in the screenshots' order.
    pub alts: &'static [&'static str],
}

/// One website's showcase (Support): PaliCanon, then Suttas Map.
#[derive(Debug)]
pub(crate) struct SiteBlock {
    /// Its heading.
    pub h: &'static str,
    /// The site's name, beside its icon.
    pub name: &'static str,
    /// One line on the site.
    pub tagline: &'static str,
    /// The button's label, linking to the site.
    pub cta: &'static str,
    /// The site's address.
    pub href: &'static str,
    /// The screenshots' alt texts, in the screenshots' order.
    pub alts: &'static [&'static str],
}

/// The `@daily_sutta` showcase (Support).
#[derive(Debug)]
pub(crate) struct InstagramBlock {
    /// Its heading.
    pub h: &'static str,
    /// One line on the account.
    pub tagline: &'static str,
    /// The Instagram button's label.
    pub cta: &'static str,
}

/// One section: a heading, then (in this order, each optional) a table,
/// a list, paragraphs and a button-styled link.
#[derive(Debug)]
pub(crate) struct Section {
    /// The `<h2>`.
    pub h: &'static str,
    /// Paragraphs.
    pub p: &'static [&'static str],
    /// Bullet items.
    pub list: &'static [&'static str],
    /// A table.
    pub table: Option<Table>,
    /// A prominent outside link (Support's Ko-fi).
    pub cta: Option<Cta>,
}

/// A table: its header cells, then its rows.
#[derive(Debug)]
pub(crate) struct Table {
    /// The header cells.
    pub head: &'static [&'static str],
    /// The rows, each a list of cells.
    pub rows: &'static [&'static [&'static str]],
}

/// A button-styled outside link.
#[derive(Debug)]
pub(crate) struct Cta {
    /// What it says.
    pub label: &'static str,
    /// Where it goes.
    pub href: &'static str,
}
'''


def fail(message: str) -> None:
    sys.exit(f"port_strings: {message}")


def snake(segment: str) -> str:
    """`noGpu` -> `no_gpu`; snake_case segments are kept."""
    return re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", segment).lower()


def flatten(value: dict, prefix: tuple = ()) -> dict:
    """Leaf strings by their dotted path, `pages` left out."""
    out = {}
    for key, child in value.items():
        path = prefix + (key,)
        if path == ("pages",):
            continue
        if isinstance(child, dict):
            out.update(flatten(child, path))
        elif isinstance(child, str):
            out[".".join(path)] = child
        else:
            fail(f"{'.'.join(path)}: neither a string nor an object")
    return out


def catalogue(lang: str, source: dict) -> dict:
    """The `leptos_i18n` catalogue of one language."""
    leaves = flatten(source)
    leaves["pages.updated"] = source["pages"]["updated"]
    out = {}
    plurals: dict = {}
    for old, value in leaves.items():
        if old in DROPPED:
            continue
        stem, suffix = old, None
        for s in PLURAL_SUFFIXES:
            if old.endswith(s):
                stem, suffix = old[: -len(s)], s
        new = "_".join(snake(part) for part in stem.split("."))
        if suffix:
            plurals.setdefault(new, {})[suffix] = value
            continue
        if new in out:
            fail(f"{lang}: two keys flatten to {new}")
        out[new] = value
    # No plural keys: leptos_i18n 0.6 deprecates ranges and its `plurals` feature pulls ICU
    # data in. As search-front's `results_one` / `results_label`, each form is a plain word
    # and the count is put before it in Rust (006: `count == 1` takes the singular).
    for new, forms in plurals.items():
        if set(forms) != set(PLURAL_SUFFIXES) or new in out:
            fail(f"{lang}: plural {new} needs exactly {PLURAL_SUFFIXES}")
        for suffix, key in ((("_one"), SINGULAR[new]), ("_other", new)):
            word = forms[suffix].replace("{{count}}", "").strip()
            if "{{" in word or key in out:
                fail(f"{lang}: plural {new}{suffix} is not '{{{{count}}}} <word>'")
            out[key] = word
    for new, value in ADDED[lang].items():
        if new in out:
            fail(f"{lang}: added key {new} already exists")
        out[new] = value
    return dict(sorted(out.items()))


def tsx_const(name: str) -> str:
    found = re.search(rf'^const {name} = "([^"]*)";$', PAGES_TSX.read_text(encoding="utf-8"), re.M)
    if not found:
        fail(f"no `const {name}` in {PAGES_TSX.relative_to(ROOT)}")
    return found.group(1)


def rehost(text: str) -> str:
    """The React site's public hosts replaced by the Leptos app's (HOSTS)."""
    return HOST_RE.sub(lambda m: HOSTS[m.group(1)], text)


def resolve(text: str, values: dict) -> str:
    text = rehost(text)
    text = re.sub(r"\{\{\s*(publisher|contact)\s*\}\}", lambda m: values[m.group(1)], text)
    if "{{" in text:
        fail(f"unresolved placeholder in a page: {text}")
    return text


class Rust:
    """Rust source for one language's pages; `rustfmt` lays it out."""

    def __init__(self, values: dict):
        self.values = values

    def s(self, text: str) -> str:
        text = resolve(text, self.values)
        if any(ord(c) < 0x20 for c in text):
            fail(f"control character in {text!r}")
        return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'

    def strs(self, items: list) -> str:
        return "&[" + "".join(self.s(i) + "," for i in items) + "]"

    def section(self, sec: dict) -> str:
        extra = set(sec) - SECTION_KEYS
        if extra:
            fail(f"unknown section keys {extra}")
        table = "None"
        if "table" in sec:
            rows = "".join(self.strs(r) + "," for r in sec["table"]["rows"])
            table = f"Some(Table {{ head: {self.strs(sec['table']['head'])}, rows: &[{rows}] }})"
        cta = "None"
        if "cta" in sec:
            cta = f"Some(Cta {{ label: {self.s(sec['cta']['label'])}, href: {self.s(sec['cta']['href'])} }})"
        return (
            f"Section {{ h: {self.s(sec['h'])}, p: {self.strs(sec.get('p', []))}, "
            f"list: {self.strs(sec.get('list', []))}, table: {table}, cta: {cta} }}"
        )

    def page(self, page: dict) -> str:
        extra = set(page) - PAGE_KEYS
        if extra:
            fail(f"unknown page keys {extra}")
        app = "None"
        if "app" in page:
            a = page["app"]
            app = (
                f"Some(AppBlock {{ h: {self.s(a['h'])}, tagline: {self.s(a['tagline'])}, "
                f"cta: {self.s(a['cta'])}, alts: {self.strs(a['alts'])} }})"
            )
        sites = "".join(
            f"SiteBlock {{ h: {self.s(w['h'])}, name: {self.s(w['name'])}, "
            f"tagline: {self.s(w['tagline'])}, cta: {self.s(w['cta'])}, "
            f"href: {self.s(w['href'])}, alts: {self.strs(w['alts'])} }},"
            for w in page.get("web", [])
        )
        instagram = "None"
        if "instagram" in page:
            i = page["instagram"]
            instagram = (
                f"Some(InstagramBlock {{ h: {self.s(i['h'])}, tagline: {self.s(i['tagline'])}, "
                f"cta: {self.s(i['cta'])} }})"
            )
        sections = "".join(self.section(sec) + "," for sec in page["sections"])
        return (
            f"PageText {{ title: {self.s(page['title'])}, description: {self.s(page['description'])}, "
            f"intro: {self.strs(page['intro'])}, app: {app}, sites: &[{sites}], "
            f"instagram: {instagram}, sections: &[{sections}] }}"
        )


def content(lang: str, pages: dict, values: dict) -> str:
    names = [k for k in pages if k != "updated"]
    if sorted(names) != sorted(PAGE_ORDER):
        fail(f"{lang}: pages {names}, expected {PAGE_ORDER}")
    rust = Rust(values)
    body = "".join(f"{name}: {rust.page(pages[name])}," for name in PAGE_ORDER)
    used = sorted(
        {"PageText", "Pages", "Section"}
        | {t for t in ("AppBlock", "Cta", "InstagramBlock", "SiteBlock", "Table") if f"{t} {{" in body}
    )
    header = f"""\
//! The footer pages in {LANG_NAMES[lang]}: generated from the React site's
//! `src/locales/{lang}.json` `pages.*` by `scripts/port_strings.py`
//! (verbatim; `{{{{publisher}}}}` and `{{{{contact}}}}` resolved as
//! `src/components/Pages.tsx` does; the hosts `viz.*` rewritten `map.*`;
//! `pages.updated` is the catalogue's `pages_updated`). Data, not code:
//! regenerate rather than edit by hand.

use super::types::{{{", ".join(used)}}};

/// Every footer page, in this language.
pub(super) const PAGES: Pages = Pages {{{body}}};
"""
    return rustfmt(header)


def rustfmt(source: str) -> str:
    if not shutil.which("rustfmt"):
        fail("rustfmt is not on the PATH")
    run = subprocess.run(
        ["rustfmt", "--edition", "2024", "--emit", "stdout"],
        input=source,
        capture_output=True,
        text=True,
    )
    if run.returncode != 0:
        fail(f"rustfmt: {run.stderr}")
    return run.stdout


def same_shape(a, b, path: str = "pages") -> None:
    """fr and vi pages hold what en's hold: same keys, same list lengths."""
    if isinstance(a, dict):
        if not isinstance(b, dict) or set(a) != set(b):
            fail(f"{path}: keys differ from en")
        for k in a:
            same_shape(a[k], b[k], f"{path}.{k}")
    elif isinstance(a, list):
        if not isinstance(b, list) or len(a) != len(b):
            fail(f"{path}: length differs from en")
        for i, (x, y) in enumerate(zip(a, b)):
            same_shape(x, y, f"{path}.{i}")
    elif not isinstance(b, str):
        fail(f"{path}: not a string")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--out", type=Path, default=ROOT, help="root to write app/ under (default: the repo)")
    out = parser.parse_args().out
    values = {"publisher": tsx_const("PUBLISHER"), "contact": tsx_const("CONTACT")}
    sources = {lang: json.loads((ROOT / "src" / "locales" / f"{lang}.json").read_text(encoding="utf-8")) for lang in LANGS}
    for lang in LANGS[1:]:
        same_shape(sources["en"]["pages"], sources[lang]["pages"], f"{lang}: pages")
    files = {Path("app/src/pages/types.rs"): rustfmt(TYPES_RS)}
    for lang in LANGS:
        cat = catalogue(lang, sources[lang])
        files[Path(f"app/locales/{lang}.json")] = json.dumps(cat, ensure_ascii=False, indent=2) + "\n"
        files[Path(f"app/src/pages/content_{lang}.rs")] = content(lang, sources[lang]["pages"], values)
    for rel, text in files.items():
        (out / rel).parent.mkdir(parents=True, exist_ok=True)
        (out / rel).write_text(text, encoding="utf-8")
        print(f"wrote {rel}")


if __name__ == "__main__":
    main()
