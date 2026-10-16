# Strings: the React catalogues → the Leptos app

The React site's i18next catalogues (`src/locales/<lang>.json`, nested) ported by
`scripts/port_strings.py` (task 005) to:

- `app/locales/<lang>.json`: the `leptos_i18n` 0.6 catalogue, flat snake_case keys, values
  verbatim, every key but `pages.*` (and `pages.updated`, kept as `pages_updated`).
- `app/src/pages/content_<lang>.rs`: `pages.*` as typed const data (`PAGES`, types in
  `app/src/pages/types.rs`), `{{publisher}}` and `{{contact}}` resolved as `Pages.tsx` does.

Regenerate with `python3 scripts/port_strings.py`; check with `python3 scripts/port_strings_test.py`.

**Plurals.** i18next's `tooltip.links_one` / `_other` become two plain keys holding the word alone,
`tooltip_link` and `tooltip_links` (vi: the same word twice), as search-front's `results_one` /
`results_label`: 006 writes `"{count} {word}"` in Rust, the singular when `count == 1`. Not
`leptos_i18n` plurals: range keys are deprecated in 0.6 (a warning, which CI denies) and suffix
plurals need the `plurals` feature and its ICU data.

**Placeholders** keep i18next's spelling, `{{count}}`; `leptos_i18n` trims the braces' contents, so
`{{count}}` and `{{ count }}` are the same variable.

## Key map, old → new

One line per leaf key of `src/locales/en.json` (fr and vi have the same keys). A `PAGES.…` target is
a field of `content_<lang>.rs`'s `PAGES`.

### `brand`

- `brand` → `brand`

### `search`

- `search.label` → `search_label`
- `search.submit` → `search_submit`
- `search.searching` → `search_searching`
- `search.placeholder` → `search_placeholder`

### `settings`

- `settings.title` → `settings_title`
- `settings.close` → `settings_close`
- `settings.appearance` → `settings_appearance`
- `settings.themes.dark` → `settings_themes_dark`
- `settings.themes.light` → `settings_themes_light`
- `settings.themes.sepia` — dropped: design v4 has no sepia theme.
- `settings.language` → `settings_language`
- `settings.language_note` → `settings_language_note`

### `views`

- `views.name` → `views_name`
- `views.liens` → `views_liens`
- `views.sim` → `views_sim`
- `views.label_sim` → `views_label_sim`
- `views.label_liens` → `views_label_liens`
- `views.label_pca` → `views_label_pca`
- `views.label_tsne` → `views_label_tsne`
- `views.label_umap` → `views_label_umap`
- `views.pca` → `views_pca`
- `views.tsne` → `views_tsne`
- `views.umap` → `views_umap`

### `legend`

- `legend.collections` → `legend_collections`
- `legend.hint` → `legend_hint`
- `legend.color` → `legend_color`
- `legend.hide` → `legend_hide`
- `legend.show` → `legend_show`
- `legend.links` → `legend_links`
- `legend.toggle` → `legend_toggle`

### `types`

- `types.full` → `types_full`
- `types.sections` → `types_sections`
- `types.resembling` → `types_resembling`
- `types.mentions` → `types_mentions`
- `types.retells` → `types_retells`
- `types.similar` → `types_similar`

### `sim`

- `sim.threshold` → `sim_threshold`
- `sim.stats` → `sim_stats`
- `sim.hide_alone` → `sim_hide_alone`

### `panel`

- `panel.read` → `panel_read`
- `panel.back` → `panel_back`
- `panel.trail` → `panel_trail`
- `panel.close` → `panel_close`

### `tooltip`

- `tooltip.links_one` → `tooltip_link` (the word alone; the count is added in Rust)
- `tooltip.links_other` → `tooltip_links` (the word alone)

### `loading`

- `loading` → `loading`

### `noGpu`

- `noGpu.title` → `no_gpu_title`
- `noGpu.body` → `no_gpu_body`

### `texts`

- `texts.name` → `texts_name`
- `texts.all` → `texts_all`
- `texts.translated` → `texts_translated`

### `credits`

- `credits.sources` → `credits_sources`
- `credits.translated` → `credits_translated`
- `credits.dpd` → `credits_dpd`
- `credits.table_title` → `credits_table_title`
- `credits.table_intro` → `credits_table_intro`
- `credits.col_lang` → `credits_col_lang`
- `credits.col_authors` → `credits_col_authors`
- `credits.col_texts` → `credits_col_texts`
- `credits.pali_edition` → `credits_pali_edition`

### `footer`

- `footer.sources` → `footer_sources` (no `t()` call site in the React code)
- `footer.app` → `footer_app`
- `footer.theme` → `footer_theme` (no `t()` call site in the React code)
- `footer.shortcut` → `footer_shortcut` (no `t()` call site in the React code)
- `footer.instagram` → `footer_instagram`
- `footer.kofi` → `footer_kofi`
- `footer.dpd` → `footer_dpd` (no `t()` call site in the React code)
- `footer.translator` → `footer_translator` (no `t()` call site in the React code)
- `footer.nav` → `footer_nav`
- `footer.nav_home` → `footer_nav_home` (no `t()` call site in the React code)
- `footer.nav_support` → `footer_nav_support`
- `footer.nav_acknowledgements` → `footer_nav_acknowledgements`
- `footer.nav_terms` → `footer_nav_terms`
- `footer.nav_privacy` → `footer_nav_privacy`
- `footer.nav_legal` → `footer_nav_legal`

### `head`

- `head.home_title` → `head_home_title`
- `head.home_description` → `head_home_description`
- `head.notfound_title` → `head_notfound_title`

### `notfound`

- `notfound.eyebrow` → `notfound_eyebrow`
- `notfound.title` → `notfound_title`
- `notfound.body` → `notfound_body`
- `notfound.home` → `notfound_home`

### `pages`

- `pages.updated` → `pages_updated` (catalogue; `{{date}}` is Pages.tsx's `UPDATED`, "2026-10-03")
- `pages.acknowledgements.title` → `PAGES.acknowledgements.title`
- `pages.acknowledgements.description` → `PAGES.acknowledgements.description`
- `pages.acknowledgements.intro` → `PAGES.acknowledgements.intro`
- `pages.acknowledgements.sections` → `PAGES.acknowledgements.sections`
- `pages.terms.title` → `PAGES.terms.title`
- `pages.terms.description` → `PAGES.terms.description`
- `pages.terms.intro` → `PAGES.terms.intro`
- `pages.terms.sections` → `PAGES.terms.sections`
- `pages.privacy.title` → `PAGES.privacy.title`
- `pages.privacy.description` → `PAGES.privacy.description`
- `pages.privacy.intro` → `PAGES.privacy.intro`
- `pages.privacy.sections` → `PAGES.privacy.sections`
- `pages.legal.title` → `PAGES.legal.title`
- `pages.legal.description` → `PAGES.legal.description`
- `pages.legal.intro` → `PAGES.legal.intro`
- `pages.legal.sections` → `PAGES.legal.sections`
- `pages.support.title` → `PAGES.support.title`
- `pages.support.description` → `PAGES.support.description`
- `pages.support.intro` → `PAGES.support.intro`
- `pages.support.app.h` → `PAGES.support.app.h`
- `pages.support.app.tagline` → `PAGES.support.app.tagline`
- `pages.support.app.cta` → `PAGES.support.app.cta`
- `pages.support.app.alts` → `PAGES.support.app.alts`
- `pages.support.instagram.h` → `PAGES.support.instagram.h`
- `pages.support.instagram.tagline` → `PAGES.support.instagram.tagline`
- `pages.support.instagram.cta` → `PAGES.support.instagram.cta`
- `pages.support.sections` → `PAGES.support.sections`
- `pages.support.web` → `PAGES.support.sites` (`[SiteBlock]`, renamed: it shows the websites)

## Call sites

Every `t()` call in `src/App.tsx` and `src/components/*.tsx`, as of this commit, with its new key.
`LangSwitch.tsx`, `main.tsx` and `i18n.ts` use i18next without `t()`.

- `src/App.tsx:80` `head.home_title` → `head_home_title`
- `src/App.tsx:80` `head.home_description` → `head_home_description`
- `src/App.tsx:392` `texts.name` → `texts_name`
- `src/App.tsx:399` `texts.all` → `texts_all`
- `src/App.tsx:400` `texts.translated` → `texts_translated`
- `src/App.tsx:414` `brand` → `brand`
- `src/App.tsx:439` `settings.title` → `settings_title`
- `src/App.tsx:440` `settings.title` → `settings_title`
- `src/App.tsx:452` `legend.toggle` → `legend_toggle`
- `src/App.tsx:495` `legend.toggle` → `legend_toggle`
- `src/App.tsx:497` `legend.toggle` → `legend_toggle`
- `src/App.tsx:503` `views.name` → `views_name`
- `src/App.tsx:504` `views.label_${mapName}` → `views_label_{sim,liens,pca,tsne,umap}` (by the current map)
- `src/App.tsx:506` `views.label_${value}` → `views_label_{sim,liens,pca,tsne,umap}` (by `MAPS[].value`)
- `src/App.tsx:506` `views.${value}` → `views_{sim,liens,pca,tsne,umap}` (by `MAPS[].value`)
- `src/App.tsx:521` `noGpu.title` → `no_gpu_title`
- `src/App.tsx:522` `noGpu.body` → `no_gpu_body`
- `src/App.tsx:526` `loading` → `loading`
- `src/App.tsx:564` `tooltip.links` → `tooltip_link` / `tooltip_links` (`count == 1` → singular)
- `src/components/Legend.tsx:30` `legend.collections` → `legend_collections`
- `src/components/Legend.tsx:31` `legend.hint` → `legend_hint`
- `src/components/Legend.tsx:41` `legend.color` → `legend_color`
- `src/components/Legend.tsx:51` `legend.hide` → `legend_hide`
- `src/components/Legend.tsx:51` `legend.show` → `legend_show`
- `src/components/Legend.tsx:52` `legend.hide` → `legend_hide`
- `src/components/Legend.tsx:52` `legend.show` → `legend_show`
- `src/components/Legend.tsx:63` `legend.links` → `legend_links`
- `src/components/Legend.tsx:72` `types.${type}` → `types_{full,sections,resembling,mentions,retells,similar}` (by link type)
- `src/components/TextPanel.tsx:46` `panel.close` → `panel_close`
- `src/components/TextPanel.tsx:47` `panel.close` → `panel_close`
- `src/components/TextPanel.tsx:52` `panel.trail` → `panel_trail`
- `src/components/TextPanel.tsx:73` `panel.read` → `panel_read`
- `src/components/TextPanel.tsx:80` `types.${graph.types[ti]}` → `types_{full,sections,resembling,mentions,retells,similar}` (by link type)
- `src/components/Translators.tsx:29` `credits.table_title` → `credits_table_title`
- `src/components/Translators.tsx:30` `credits.table_intro` → `credits_table_intro`
- `src/components/Translators.tsx:35` `credits.col_lang` → `credits_col_lang`
- `src/components/Translators.tsx:36` `credits.col_authors` → `credits_col_authors`
- `src/components/Translators.tsx:37` `credits.col_texts` → `credits_col_texts`
- `src/components/Translators.tsx:43` `credits.pali_edition` → `credits_pali_edition`
- `src/components/SearchBar.tsx:47` `search.label` → `search_label`
- `src/components/SearchBar.tsx:56` `search.placeholder` → `search_placeholder`
- `src/components/SearchBar.tsx:64` `search.submit` → `search_submit`
- `src/components/SearchBar.tsx:67` `search.searching` → `search_searching`
- `src/components/SearchBar.tsx:67` `search.submit` → `search_submit`
- `src/components/Footer.tsx:36` `footer.${key}` → `footer_app`, `footer_instagram`, `footer_kofi` (by `LINKS[].key`)
- `src/components/Footer.tsx:37` `footer.${key}` → `footer_app`, `footer_instagram`, `footer_kofi` (by `LINKS[].key`)
- `src/components/Footer.tsx:76` `footer.nav` → `footer_nav`
- `src/components/Footer.tsx:79` `footer.nav_${key}` → `footer_nav_acknowledgements`, `footer_nav_terms`, `footer_nav_privacy`, `footer_nav_legal`, `footer_nav_support` (by `NAV[].key`)
- `src/components/SettingsPanel.tsx:34` `settings.title` → `settings_title`
- `src/components/SettingsPanel.tsx:35` `settings.close` → `settings_close`
- `src/components/SettingsPanel.tsx:42` `settings.appearance` → `settings_appearance`
- `src/components/SettingsPanel.tsx:43` `settings.appearance` → `settings_appearance`
- `src/components/SettingsPanel.tsx:54` `settings.themes.${th.id}` → `settings_themes_dark`, `settings_themes_light` (sepia dropped)
- `src/components/SettingsPanel.tsx:61` `settings.language` → `settings_language`
- `src/components/SettingsPanel.tsx:62` `settings.language` → `settings_language`
- `src/components/SettingsPanel.tsx:77` `settings.language_note` → `settings_language_note`
- `src/components/SimControls.tsx:19` `sim.threshold` → `sim_threshold`
- `src/components/SimControls.tsx:33` `sim.stats` → `sim_stats`
- `src/components/SimControls.tsx:37` `sim.hide_alone` → `sim_hide_alone`
- `src/components/Pages.tsx:105` `pages.${kind}` → `PAGES.<kind>` of `content_<lang>.rs` (whole page, `{{publisher}}`/`{{contact}}` resolved)
- `src/components/Pages.tsx:114` `panel.back` → `panel_back`
- `src/components/Pages.tsx:122` `footer.nav` → `footer_nav`
- `src/components/Pages.tsx:125` `footer.nav_${k}` → `footer_nav_acknowledgements`, `footer_nav_terms`, `footer_nav_privacy`, `footer_nav_legal`, `footer_nav_support` (by `PageKind`)
- `src/components/Pages.tsx:183` `pages.updated` → `pages_updated`
- `src/components/Pages.tsx:192` `head.notfound_title` → `head_notfound_title`
- `src/components/Pages.tsx:197` `brand` → `brand`
- `src/components/Pages.tsx:199` `notfound.eyebrow` → `notfound_eyebrow`
- `src/components/Pages.tsx:200` `notfound.title` → `notfound_title`
- `src/components/Pages.tsx:201` `notfound.body` → `notfound_body`
- `src/components/Pages.tsx:203` `notfound.home` → `notfound_home`
- `src/components/Credits.tsx:11` `credits.sources` → `credits_sources`
- `src/components/Credits.tsx:15` `credits.translated` → `credits_translated`
- `src/components/Credits.tsx:21` `credits.dpd` → `credits_dpd`
