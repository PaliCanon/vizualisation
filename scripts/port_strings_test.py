"""Checks the strings ported by `port_strings.py` (task 005), stdlib only:

    python3 scripts/port_strings_test.py

- the three catalogues (`app/locales/<lang>.json`) have the same key set, every key a plain
  snake_case identifier that `leptos_i18n` reads as itself (no stray plural suffix);
- each key has the same `{{ name }}` placeholders in en, fr and vi;
- every leaf key of the React catalogues (`src/locales/<lang>.json`) is mapped in `app/STRINGS.md`
  or listed there as dropped, with a reason;
- the page modules (`app/src/pages/content_<lang>.rs`) hold every page title of their language,
  and no unresolved placeholder;
- the committed outputs are what the generator writes today (regenerate rather than edit).
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANGS = ("en", "fr", "vi")
OLD = ROOT / "src" / "locales"
APP = ROOT / "app"
STRINGS = APP / "STRINGS.md"

# The forms `leptos_i18n` 0.6 merges into one plural key when a key ends with them.
PLURAL_SUFFIX = re.compile(r"_(zero|one|two|few|many|other)$")
RUST_KEYWORDS = {
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait", "true",
    "type", "unsafe", "use", "where", "while",
}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def placeholders(value) -> set[str]:
    """`{{ name }}` names in a string, or in every string of a range (a list of arms)."""
    if isinstance(value, str):
        return {m.split(",")[0].strip() for m in re.findall(r"\{\{(.*?)\}\}", value)}
    if isinstance(value, list):
        return set().union(*(placeholders(v) for v in value))
    return set()


def old_leaves(value, prefix: str = ""):
    """The React catalogue's leaf keys: strings and arrays (pages' lists) are leaves."""
    if isinstance(value, dict):
        for key, child in value.items():
            yield from old_leaves(child, f"{prefix}.{key}" if prefix else key)
    else:
        yield prefix


class Catalogues(unittest.TestCase):
    def setUp(self):
        self.cats = {lang: load(APP / "locales" / f"{lang}.json") for lang in LANGS}

    def test_same_key_set(self):
        en = set(self.cats["en"])
        for lang in LANGS[1:]:
            self.assertEqual(set(self.cats[lang]) ^ en, set(), lang)

    def test_keys_are_plain_identifiers(self):
        for lang, cat in self.cats.items():
            for key in cat:
                self.assertRegex(key, r"^[a-z][a-z0-9_]*$", f"{lang}: {key}")
                self.assertNotIn(key, RUST_KEYWORDS, f"{lang}: {key}")
                self.assertIsNone(PLURAL_SUFFIX.search(key), f"{lang}: {key} would be merged as a plural")

    def test_same_placeholders(self):
        for key, value in self.cats["en"].items():
            for lang in LANGS[1:]:
                self.assertEqual(placeholders(self.cats[lang][key]), placeholders(value), f"{lang}: {key}")

    def test_only_ranges_are_arrays(self):
        """An array is read as a plural range: each arm `[text, count…]`, the last one `"_"`."""
        for lang, cat in self.cats.items():
            for key, value in cat.items():
                if isinstance(value, str):
                    continue
                self.assertIsInstance(value, list, f"{lang}: {key}")
                for arm in value:
                    self.assertIsInstance(arm[0], str, f"{lang}: {key}")
                self.assertEqual(value[-1][1:], ["_"], f"{lang}: {key} needs a fallback arm")

    def test_sorted_two_space_trailing_newline(self):
        for lang, cat in self.cats.items():
            text = (APP / "locales" / f"{lang}.json").read_text(encoding="utf-8")
            self.assertEqual(list(cat), sorted(cat), lang)
            self.assertTrue(text.endswith("}\n"), lang)
            self.assertTrue(text.startswith('{\n  "'), lang)


class KeyMap(unittest.TestCase):
    def test_every_old_key_is_mapped_or_dropped(self):
        lines = STRINGS.read_text(encoding="utf-8").splitlines()
        new_keys = set(load(APP / "locales" / "en.json"))
        for lang in LANGS:
            for key in old_leaves(load(OLD / f"{lang}.json")):
                found = [line for line in lines if line.lstrip("- ").startswith(f"`{key}` ")]
                self.assertEqual(len(found), 1, f"{lang}: {key} is mapped {len(found)} times")
                line = found[0]
                mapped = re.search(r"→ `([^`]+)`", line)
                dropped = re.search(r"dropped: \S", line)
                self.assertTrue(mapped or dropped, f"{lang}: {key}: {line}")
                if mapped and not mapped.group(1).startswith("PAGES."):
                    self.assertIn(mapped.group(1), new_keys, line)


class Content(unittest.TestCase):
    def test_every_title_in_its_language(self):
        for lang in LANGS:
            pages = load(OLD / f"{lang}.json")["pages"]
            rust = (APP / "src" / "pages" / f"content_{lang}.rs").read_text(encoding="utf-8")
            for name, page in pages.items():
                if name == "updated":
                    continue
                title = page["title"].replace("\\", "\\\\").replace('"', '\\"')
                self.assertIn(f'title: "{title}",', rust, f"{lang}: {name}")
                self.assertIn(f"    {name}: PageText {{", rust, f"{lang}: {name}")

    def test_no_placeholder_left(self):
        for lang in LANGS:
            rust = (APP / "src" / "pages" / f"content_{lang}.rs").read_text(encoding="utf-8")
            data = [line for line in rust.splitlines() if not line.lstrip().startswith("//")]
            left = [line.strip()[:80] for line in data if "{{" in line]
            self.assertEqual(left, [], lang)


class Regenerated(unittest.TestCase):
    def test_outputs_are_current(self):
        with tempfile.TemporaryDirectory() as out:
            run = subprocess.run(
                [sys.executable, str(ROOT / "scripts" / "port_strings.py"), "--out", out],
                capture_output=True,
                text=True,
            )
            self.assertEqual(run.returncode, 0, run.stderr)
            written = sorted(p.relative_to(out) for p in Path(out).rglob("*") if p.is_file())
            self.assertTrue(written)
            for rel in written:
                self.assertEqual(
                    (Path(out) / rel).read_text(encoding="utf-8"),
                    (ROOT / rel).read_text(encoding="utf-8"),
                    f"{rel} differs from a fresh run: regenerate it",
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)
