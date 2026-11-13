"""No JavaScript in the repository (task 008, then task 022).

The React site, its Node data tools and the Pages function were removed
(user, 2026-10-22: "je pense quon peut virer le js"); the Cloudflare Worker
in front of Cloud Run, the one file kept then, went with the dev
environment (task 022). The test reads the
tracked files (`git ls-files`), so an untracked build output
(`target/site/pkg/*.js`, wasm-bindgen's) is not counted.

Mutations it catches: a tracked `.js`, `.mjs`, `.cjs`, `.ts`, `.tsx` or
`.jsx` file anywhere, a `package.json`, a lock of npm, a
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

JS = re.compile(r"\.(js|mjs|cjs|ts|tsx|jsx)$")
NODE_FILES = re.compile(r"(^|/)(package\.json|package-lock\.json|tsconfig[^/]*\.json|vite\.config\.[a-z]+)$")


def offenders(paths):
    """The paths of `paths` that are JavaScript, TypeScript or Node files."""
    return sorted(p for p in paths if (JS.search(p) or NODE_FILES.search(p)))


def tracked():
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, check=True, capture_output=True, text=True)
    return out.stdout.splitlines()


class NoJavaScript(unittest.TestCase):
    def test_there_is_no_javascript(self):
        self.assertEqual(offenders(tracked()), [])

    def test_each_kind_is_caught(self):
        caught = [
            "src/main.tsx", "src/a.ts", "x/b.jsx", "scripts/c.mjs", "layout/d.cjs", "functions/e.js",
            "package.json", "package-lock.json", "tsconfig.node.json", "vite.config.ts",
        ]
        self.assertEqual(offenders(caught), sorted(caught))

    def test_rust_and_data_pass(self):
        kept = ["app/src/lib.rs", "public/graph.json"]
        self.assertEqual(offenders(kept), [])

    def test_the_removed_worker_is_caught_if_it_comes_back(self):
        self.assertEqual(offenders(["infra/worker/proxy.js", "infra/worker/proxy.test.mjs"]),
                         ["infra/worker/proxy.js", "infra/worker/proxy.test.mjs"])


if __name__ == "__main__":
    unittest.main()
