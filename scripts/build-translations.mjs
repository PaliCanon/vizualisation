// Builds, from SuttaCentral only (fetched by scripts/fetch-data.sh):
//   public/translations.json  the texts of the Pāli canon translated into each language of the app,
//                             with title and translator —
//                             { en: { titles: { uid: title }, by: { uid: i }, translators: [name] }, fr, vi }
//   public/blurbs-<lang>.json SuttaCentral's summary of each text, where it has one: { uid: blurb }
//
// Translations:
//   data/bilara-data/translation/<lang>/<translator>/...  current segmented translations; title = the
//       header segment that holds the Pāli title in root/pli/ms (else the last one); translator's name
//       from sc_bilara_data/_author.json
//   data/sc-data/html_text/<lang>/pli/...                  legacy HTML translations; title = first <h1>
//       (what SuttaCentral shows), number and trailing colon dropped; translator = <meta name="author">
//   Bilara wins when a text has both.
// Blurbs: sc_bilara_data/root/en/blurb (English), sc_bilara_data/translation/<lang>/blurb (others).
import fs from 'node:fs';
import path from 'node:path';

const LANGS = ['en', 'fr', 'vi'];
const SC = 'data/sc-data/sc_bilara_data';
const walk = (dir) => (fs.existsSync(dir)
  ? fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(path.join(dir, e.name)) : [path.join(dir, e.name)]))
  : []);
const clean = (s) => s.replace(/<[^>]+>/g, '').replace(/&nbsp;/g, ' ').replace(/\s+/g, ' ').trim();
const readJson = (f) => JSON.parse(fs.readFileSync(f, 'utf8'));

// Which header segment is the title: the one whose Pāli root text is the text's Pāli name
// (sc_bilara_data/root/.../name). Headers also hold the collection, chapter or story.
const rootNames = new Map();
for (const f of walk(`${SC}/root`)) {
  if (!f.includes(`${path.sep}name${path.sep}`)) continue;
  for (const [k, v] of Object.entries(readJson(f))) {
    const uid = k.split(':')[1]?.replace(/^[\d.]+?\./, '');
    if (uid && v.trim()) rootNames.set(uid, clean(v));
  }
}
const titleKey = new Map();
for (const f of walk('data/bilara-data/root/pli/ms')) {
  if (!f.endsWith('_root-pli-ms.json')) continue;
  const uid = path.basename(f).replace(/_root-pli-ms\.json$/, '');
  const name = rootNames.get(uid);
  const header = Object.entries(readJson(f)).filter(([k]) => /:0\.\d+$/.test(k));
  const hit = name && header.find(([, v]) => clean(v) === name);
  if (hit) titleKey.set(uid, hit[0]);
}
const authors = Object.fromEntries(Object.entries(readJson(`${SC}/_author.json`)).map(([k, v]) => [k, v.name]));
// _author.json names the bilara translator `phantuananh` "Bhikkhu Thích Minh Châu"; the texts are
// Phan Tuấn Anh's (SuttaCentral's site and the translator id agree on that).
authors.phantuananh = 'Phan Tuấn Anh';

const out = {};
for (const lang of LANGS) {
  const titles = {};
  const by = {};
  const translators = [];
  const credit = (uid, name) => {
    if (!name) return;
    let i = translators.indexOf(name);
    if (i < 0) i = translators.push(name) - 1;
    by[uid] = i;
  };
  let fromBilara = 0;
  let fromHtml = 0;

  // Bilara: translation/<lang>/<translator>/<sutta|vinaya|abhidhamma>/.../<uid>_translation-<lang>-<translator>.json
  for (const f of walk(`data/bilara-data/translation/${lang}`)) {
    const parts = f.split(path.sep);
    // data / bilara-data / translation / <lang> / <translator> / <sutta|vinaya|...|name|blurb|site|comment>
    const translator = parts[4];
    if (!f.endsWith('.json') || [translator, parts[5]].some((p) => ['name', 'blurb', 'site', 'comment'].includes(p))) continue;
    const uid = path.basename(f).replace(/_translation-.*$/, '');
    if (uid in titles) continue;
    const segments = readJson(f);
    const header = Object.entries(segments).filter(([k, v]) => /:0\.\d+$/.test(k) && v.trim());
    titles[uid] = clean(segments[titleKey.get(uid)] ?? header.at(-1)?.[1] ?? '');
    credit(uid, authors[translator] ?? translator);
    fromBilara++;
  }

  // Legacy HTML: html_text/<lang>/pli/.../<uid>.html (pli = translations of the Pāli canon)
  for (const f of walk(`data/sc-data/html_text/${lang}/pli`)) {
    if (!f.endsWith('.html')) continue;
    const uid = path.basename(f, '.html');
    if (uid in titles) continue;
    const html = fs.readFileSync(f, 'utf8');
    const h1 = html.match(/<h1[^>]*>([\s\S]*?)<\/h1>/);
    titles[uid] = h1 ? clean(h1[1]).replace(/^[\d.\s–-]+(?=\S)/, '').replace(/\s*:$/, '') : '';
    credit(uid, html.match(/<meta name=['"]author['"] content=['"]([^'"]+)['"]/)?.[1]?.trim());
    fromHtml++;
  }

  out[lang] = { titles, by, translators };
  console.log(`${lang}: ${Object.keys(titles).length} texts (${fromBilara} bilara, ${fromHtml} legacy html), `
    + `${translators.length} translators, ${Object.values(titles).filter((t) => !t).length} without title`);

  // Blurbs
  const blurbDir = lang === 'en' ? `${SC}/root/en/blurb` : `${SC}/translation/${lang}/blurb`;
  const blurbs = {};
  for (const f of walk(blurbDir)) {
    for (const [k, v] of Object.entries(readJson(f))) {
      const uid = k.split(':')[1];
      if (uid && clean(v)) blurbs[uid] = clean(v);
    }
  }
  if (Object.keys(blurbs).length) {
    fs.writeFileSync(`public/blurbs-${lang}.json`, JSON.stringify(blurbs));
    console.log(`  ${Object.keys(blurbs).length} blurbs -> public/blurbs-${lang}.json`);
  }
}
fs.writeFileSync('public/translations.json', JSON.stringify(out));
console.log(`wrote public/translations.json (${(fs.statSync('public/translations.json').size / 1e3).toFixed(0)} kB)`);
