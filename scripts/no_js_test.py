"""No JavaScript in the repository but the Cloudflare Worker (task 008).

The React site, its Node data tools and the Pages function were removed
(user, 2026-10-22: "je pense quon peut virer le js"); the one JavaScript
file that stays is the Worker in front of Cloud Run, with its `node --test`
tests, as in learn-pali, search-front and the API. The test reads the
tracked files (`git ls-files`), so an untracked build output
(`target/site/pkg/*.js`, wasm-bindgen's) is not counted.

Mutations it catches: a tracked `.js`, `.mjs`, `.cjs`, `.ts`, `.tsx` or
`.jsx` file anywhere but the Worker's, a `package.json`, a lock of npm, a
`tsconfig*.json`, a Vite config. Not caught: JavaScript inlined in a Rust
string or in HTML (the app's `view!` scripts are its own business), and a
tracked `node_modules/` file whose extension is none of those.

  python3 scripts/no_js_test.py
"""

import pathlib
import re
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parent.parent

# The Worker and its tests: the only JavaScript kept.
ALLOWED = re.compile(r"^infra/worker/(proxy\.js|[^/]+\.test\.mjs)$")
JS = re.compile(r"\.(js|mjs|cjs|ts|tsx|jsx)$")
NODE_FILES = re.compile(r"(^|/)(package\.json|package-lock\.json|tsconfig[^/]*\.json|vite\.config\.[a-z]+)$")


def offenders(paths):
    """The paths of `paths` that are JavaScript, TypeScript or Node files, the Worker's apart."""
    return sorted(p for p in paths if (JS.search(p) or NODE_FILES.search(p)) and not ALLOWED.match(p))


def tracked():
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, check=True, capture_output=True, text=True)
    return out.stdout.splitlines()


class NoJavaScript(unittest.TestCase):
    def test_the_worker_is_the_only_javascript(self):
        self.assertEqual(offenders(tracked()), [])

    def test_the_worker_and_its_tests_are_still_there(self):
        files = tracked()
        self.assertIn("infra/worker/proxy.js", files)
        self.assertTrue(any(ALLOWED.match(f) and f.endswith(".test.mjs") for f in files))

    def test_each_kind_is_caught(self):
        caught = [
            "src/main.tsx", "src/a.ts", "x/b.jsx", "scripts/c.mjs", "layout/d.cjs", "functions/e.js",
            "package.json", "package-lock.json", "tsconfig.node.json", "vite.config.ts",
        ]
        self.assertEqual(offenders(caught), sorted(caught))

    def test_the_worker_its_tests_and_rust_pass(self):
        kept = ["infra/worker/proxy.js", "infra/worker/proxy.test.mjs", "app/src/lib.rs", "public/graph.json"]
        self.assertEqual(offenders(kept), [])

    def test_a_js_file_elsewhere_in_infra_is_caught(self):
        self.assertEqual(offenders(["infra/worker/helper.js", "infra/other/proxy.js"]),
                         ["infra/other/proxy.js", "infra/worker/helper.js"])


if __name__ == "__main__":
    unittest.main()
