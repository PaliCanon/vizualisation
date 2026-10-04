"""The vectorial space placed by its own coordinates: the LSA vectors (512-d) reduced to 3D, instead
of the force layout of graph-sim.json. Same texts, same similarity pairs (graph-sim.bin), other
positions; `fixed: true` tells the page not to re-run the layout when the threshold moves.

  pca    linear, the three directions of largest variance
  tsne   t-SNE (cosine, perplexity 30): keeps neighbourhoods, pulls groups apart
  umap   UMAP (cosine, 15 neighbours): neighbourhoods and some of the global shape

Run with pali-embedder's environment (scikit-learn, umap-learn):
    ../pali-embedder/.venv/bin/python scripts/reduce-3d.py [pca tsne umap]
-> public/graph-sim-<method>.json, scaled like graph-sim.json (95th-percentile radius).
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
EMBEDDER = ROOT.parent / "pali-embedder" if (ROOT.parent / "pali-embedder").exists() else ROOT.parent / "palicanon/pali-embedder"
sys.path.insert(0, str(EMBEDDER))
from pali_embedder import vectors  # noqa: E402

VEC = EMBEDDER / "out/pali-lsa-kept-512"


def reduce(method, X):
    if method == "pca":
        from sklearn.decomposition import PCA
        return PCA(n_components=3, random_state=0).fit_transform(X)
    if method == "tsne":
        from sklearn.manifold import TSNE
        return TSNE(n_components=3, metric="cosine", perplexity=30, init="pca", random_state=0).fit_transform(X)
    if method == "umap":
        import umap
        return umap.UMAP(n_components=3, metric="cosine", n_neighbors=15, min_dist=0.1, random_state=0).fit_transform(X)
    raise SystemExit(f"unknown method {method}")


def main():
    methods = sys.argv[1:] or ["pca", "tsne", "umap"]
    base = json.loads((ROOT / "public/graph-sim.json").read_text())
    uids, V = vectors.read(VEC.with_suffix(".bin"), 512, VEC.with_suffix(".json"))
    pos = {u: i for i, u in enumerate(uids)}
    X = np.asarray(V, dtype=np.float64)[[pos[n[0]] for n in base["nodes"]]]
    old = np.array([[n[7], n[8], n[9]] for n in base["nodes"]])
    target = np.percentile(np.linalg.norm(old - np.median(old, axis=0), axis=1), 95)
    for m in methods:
        t0 = time.time()
        P = reduce(m, X)
        P = P - np.median(P, axis=0)
        P *= target / np.percentile(np.linalg.norm(P, axis=1), 95)
        g = dict(base)
        g["nodes"] = [n[:7] + [round(float(x), 1) for x in p] for n, p in zip(base["nodes"], P)]
        g["fixed"] = True
        (ROOT / f"public/graph-sim-{m}.json").write_text(json.dumps(g, ensure_ascii=False, separators=(",", ":")))
        print(f"{m}: {time.time() - t0:.0f}s -> public/graph-sim-{m}.json", flush=True)


if __name__ == "__main__":
    main()
