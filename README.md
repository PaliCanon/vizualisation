# vizualisation — Suttas Map

Carte 3D du canon pāli, servie sur [viz.palicanon.app](https://viz.palicanon.app) : les parallèles de
SuttaCentral, et un espace vectoriel construit sur le texte pāli.

**Contient** : le site (`src/`, React + Tailwind), le moteur 3D (`engine/`, Rust → WebAssembly, générique :
il ne connaît aucun texte), les scripts qui fabriquent les données (`scripts/`) et ces données (`public/`),
l'infra Cloudflare (`infra/`). **Ne contient jamais** : les clones de SuttaCentral (`data/`, refaits par
`npm run fetch-data`), les vecteurs de pali-embedder (lus dans `../pali-embedder` au moment de
`npm run build-sim`), de secret.

```sh
npm install
npm run fetch-data          # SuttaCentral : sc-data + bilara-data (clones partiels dans data/)
npm run build-graph         # vue SuttaCentral    -> public/graph.json
npm run build-sim           # vue Vectorial space -> public/graph-sim.json + .bin
npm run build-translations  # textes traduits     -> public/translations.json
npm run build:engine        # moteur 3D (Rust -> wasm) -> engine/pkg
npm run dev
```

> Si npm échoue avec `UNABLE_TO_GET_ISSUER_CERT_LOCALLY`, préfixer par
> `NODE_EXTRA_CA_CERTS=/etc/ssl/cert.pem`.

## Données

Tout le contenu affiché vient de **SuttaCentral** ([sc-data](https://github.com/suttacentral/sc-data),
[bilara-data](https://github.com/suttacentral/bilara-data)). Seule exception : les positions de la vue
« Vectorial space », calculées par [pali-embedder](https://github.com/Lsh0x/pali-embedder) sur le texte pāli
de SuttaCentral, avec le *Digital Pāḷi Dictionary* pour ramener chaque mot à sa forme de base.

| Fichier | Contenu | Source |
|---|---|---|
| `public/graph.json` | Textes pāli ayant un parallèle, liens par type, positions 3D (simulation physique) | sc-data `relationship/new_parallels.json` |
| `public/graph-sim.json`, `.bin` | Tout le canon (7 286 textes), paires de textes par similarité | pali-embedder : bilara-data `root/pli/ms` + DPD |
| `public/translations.json` | Textes traduits en en / fr / vi sur SuttaCentral, avec leur titre | bilara-data `translation/`, sc-data `html_text/*/pli` |
| Sigles, titres pāli et anglais, collections | | sc-data `structure/`, `misc/`, `sc_bilara_data/*/name` |

Titres traduits : pour bilara, le segment d'en-tête qui porte le titre pāli du texte ; pour les anciennes
traductions HTML, le premier `<h1>` (ce qu'affiche SuttaCentral).

## Vues

- **SuttaCentral** : les textes sont placés par les parallèles de SuttaCentral (ressorts entre textes liés).
- **Vectorial space** : même simulation, mais les liens sont les paires de textes dont la similarité
  (vecteurs pāli, 512 dimensions) dépasse un seuil réglable ; la disposition est recalculée en direct.
- **Textes** : Pāli (tout le canon) ou une langue (seulement les textes traduits sur SuttaCentral, titres
  traduits). Suit la langue de l'interface tant qu'on n'en choisit pas une autre.

URL partageable : `?map=sim&th=0.75#mn10`. La navigation entre textes passe par l'historique du navigateur
(bouton précédent, fil d'Ariane dans le panneau).

## Code

- Interface : **React 19 + Tailwind v4**, même stack et même thème que palicanon.app (`palisearch/web`).
  `src/styles.css`, `src/icons.tsx`, `src/i18n.ts`, `src/settings.ts` et
  `src/components/{SearchBar,Menu,LangSwitch,SettingsPanel}.tsx` sont **copiés** de `palisearch/web/src`
  (commit noté en tête de fichier) : les garder synchronisés. Mêmes clés de stockage (`theme`,
  `palisearch.lang`) : intégrée à palicanon.app, la carte partage ces choix.
- `src/map/engine.ts` : la carte Three.js, sans interface ; `src/App.tsx` la pilote.
- `scripts/` : construction des données ; `scripts/lib/canon.mjs` : sigles, titres et collections depuis sc-data.
- Les vecteurs de la vue Vectorial space sont lus dans `../pali-embedder/out/pali-lsa-kept-512.*` (LSA pāli avec formules, bigrammes rares et WCCN : `python -m pali_embedder.lsa.variants --write`, voir `out/lsa-variants.md`).
- Référencement : `index.html` porte la description, Open Graph (`public/og.png`, 1200×630, tiré de
  `public/support/web/map-whole.jpg`), le JSON-LD et un `<noscript>` lisible ; `public/robots.txt`,
  `public/sitemap.xml` (la carte et les 5 pages, à tenir à jour), `public/_redirects` (toute adresse →
  `index.html`, la 404 est rendue côté client avec `noindex`). Titre et canonique par route : `src/head.ts`.

## Déploiement

Site statique sur **Cloudflare Pages** (projet `palicanon-viz`), comme palicanon.app :

- **push sur `main` → dev** : [dev.viz.palicanon.app](https://dev.viz.palicanon.app), protégé par un mot de passe
  (`functions/_middleware.js`, secret Pages `DEV_PASSWORD` sur l'environnement preview seulement, posé par l'API
  Cloudflare depuis le trousseau : compte `palicanon`, service `viz-dev-password`) ;
- **tag `v*` → prod** : [viz.palicanon.app](https://viz.palicanon.app).

`.github/workflows/deploy.yml` construit le moteur et le site (`.github/actions/build`) puis
`wrangler pages deploy` (secrets `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`).
`.github/workflows/ci.yml` vérifie chaque PR : le moteur (fmt, clippy, tests, rustdoc, MSRV, cargo-deny,
couverture → Codecov) et le site (tsc, build).

L'infra est en OpenTofu dans `infra/`, appliquée à la main (son propre état, préfixe `suttasmap` dans
le bucket de palicanon) : le projet Pages, les deux domaines (Pages fournit le certificat de `dev.viz`, deux niveaux de
sous-domaine) et leur DNS. Appliquée depuis le Mac, comme celle de palicanon.

```sh
cd infra
cp terraform.tfvars.example terraform.tfvars        # id du compte Cloudflare
gcloud auth application-default login               # le bucket d'état
export CLOUDFLARE_API_TOKEN=$(security find-generic-password -a palicanon -s cloudflare-api-token -w)
tofu init && tofu apply
```
