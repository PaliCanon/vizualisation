import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useHead } from "./head";
import { LangSwitch } from "./components/LangSwitch";
import { SettingsPanel } from "./components/SettingsPanel";
import { GearIcon, PanelIcon } from "./icons";
import { loadTheme, type Theme } from "./settings";
import { mapTheme } from "./map/theme";
import { SearchBar } from "./components/SearchBar";
import { Menu } from "./components/Menu";
import { Legend } from "./components/Legend";
import { Footer, SocialLinks } from "./components/Footer";
import { SimControls } from "./components/SimControls";
import { TextPanel, TextRow } from "./components/TextPanel";
import { SuttaRef, type RefData } from "./components/SuttaRef";
import { MapEngine, type EngineEvents } from "./map/engine";
import { RustMapEngine } from "./map/rust-engine";
import { MAPS, linksAt, loadBlurbs, loadGraph, loadPairs, loadTranslations, normalize, type Graph, type MapName, type Pairs, type Translations } from "./map/data";

const card = "rounded-xl border border-card-edge bg-card shadow-[0_16px_40px_-16px_rgb(0_0_0/0.7)]";

// Coloured / hidden collections survive switching view and reloading.
const LEGEND_KEY = "suttasmap.legend";
function loadLegend(): { colored: Set<string>; hidden: Set<string> } {
  try {
    const s = JSON.parse(localStorage.getItem(LEGEND_KEY) ?? "{}");
    return { colored: new Set(s.colored ?? []), hidden: new Set(s.hidden ?? []) };
  } catch {
    return { colored: new Set(), hidden: new Set() };
  }
}

// Language of the texts on the map: Pāli = the whole canon; otherwise only the texts
// translated into that language, with their translated title.
const TEXT_LANGS = [
  { value: "pli", label: "Pāli" },
  { value: "en", label: "English" },
  { value: "fr", label: "Français" },
  { value: "vi", label: "Tiếng Việt" },
] as const;
type TextLang = (typeof TEXT_LANGS)[number]["value"];
const TEXT_LANG_KEY = "suttasmap.textLang";

function useMediaQuery(query: string) {
  const [match, setMatch] = useState(() => window.matchMedia(query).matches);
  useEffect(() => {
    const m = window.matchMedia(query);
    const on = () => setMatch(m.matches);
    m.addEventListener("change", on);
    return () => m.removeEventListener("change", on);
  }, [query]);
  return match;
}

// SuttaCentral is the default view; ?map=sim opens the vectorial space.
// The map's engine: Rust (engine/, WebGPU) by default; ?engine=three for the three.js one, which
// is also the fallback when the Rust one cannot start.
type Engine = MapEngine | RustMapEngine;
const useThree = new URLSearchParams(location.search).get("engine") === "three";
async function createEngine(canvas: HTMLCanvasElement, graph: Graph, theme: ReturnType<typeof mapTheme>, events: EngineEvents): Promise<Engine> {
  if (!useThree) {
    try {
      return await RustMapEngine.create(canvas, graph, theme, events);
    } catch (err) {
      console.warn("Rust map engine unavailable, using three.js", err);
    }
  }
  return new MapEngine(canvas, graph, theme, events);
}

const initialMap = (): MapName => (new URLSearchParams(location.search).get("map") === "sim" ? "sim" : "liens");

export function App() {
  const { t, i18n } = useTranslation();
  useHead({ title: t("head.home_title"), description: t("head.home_description"), canonical: "/" });
  const [translations, setTranslations] = useState<Translations | null>(null);
  useEffect(() => { void loadTranslations().then(setTranslations); }, []);
  // Texts language: follows the interface language until picked in the chip; changing the
  // interface language makes it follow again.
  const uiLang = (i18n.resolvedLanguage ?? "en") as TextLang;
  const [textLangChoice, setTextLangChoice] = useState<TextLang | null>(() => {
    try { return localStorage.getItem(TEXT_LANG_KEY) as TextLang | null; } catch { return null; }
  });
  const firstUiLang = useRef(uiLang);
  useEffect(() => {
    if (uiLang === firstUiLang.current) return;
    firstUiLang.current = uiLang;
    setTextLangChoice(null);
  }, [uiLang]);
  useEffect(() => {
    try {
      if (textLangChoice) localStorage.setItem(TEXT_LANG_KEY, textLangChoice);
      else localStorage.removeItem(TEXT_LANG_KEY);
    } catch { /* private mode */ }
  }, [textLangChoice]);
  const textLang: TextLang = textLangChoice ?? uiLang;
  const langTitles = textLang === "pli" ? null : (translations?.[textLang]?.titles ?? null);
  // Glosses and summaries in the texts' language; English ones when the texts are in Pāli, and for
  // the summaries SuttaCentral has not translated.
  const glossLang = textLang === "pli" ? "en" : textLang;
  const [blurbs, setBlurbs] = useState<{ lang: Record<string, string>; en: Record<string, string> }>({ lang: {}, en: {} });
  useEffect(() => {
    let live = true;
    void Promise.all([loadBlurbs(glossLang), glossLang === "en" ? null : loadBlurbs("en")]).then(([lang, en]) => {
      if (live) setBlurbs({ lang, en: en ?? lang });
    });
    return () => { live = false; };
  }, [glossLang]);
  const [settingsOpen, setSettingsOpen] = useState(false);
  // The collections panel can be closed; the choice is remembered on wide screens. On narrow ones
  // (below md) it starts closed and opens over the map, until the map or a text is touched.
  const narrow = useMediaQuery("(max-width: 767px)");
  const [legendOpen, setLegendOpen] = useState(() => {
    if (window.matchMedia("(max-width: 767px)").matches) return false;
    try { return localStorage.getItem("suttasmap.legendOpen") !== "0"; } catch { return true; }
  });
  useEffect(() => {
    if (narrow) return;
    try { localStorage.setItem("suttasmap.legendOpen", legendOpen ? "1" : "0"); } catch { /* private mode */ }
  }, [legendOpen, narrow]);
  useEffect(() => { if (narrow) setLegendOpen(false); }, [narrow]);
  // The settings panel (copied from palisearch) writes <html data-theme>; follow it.
  const [theme, setTheme] = useState<Theme>(loadTheme);
  useEffect(() => {
    const obs = new MutationObserver(() => setTheme(loadTheme()));
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => obs.disconnect();
  }, []);
  const [mapName, setMapName] = useState<MapName>(initialMap);
  const [graph, setGraph] = useState<Graph | null>(null);
  const [pairs, setPairs] = useState<Pairs | null>(null);
  const [engineState, setEngine] = useState<Engine | null>(null);
  const [shownGraph, setShownGraph] = useState<Graph | null>(null); // the graph the engine draws
  // The engine, once it draws the current graph (between views it still draws the previous one).
  const engine = engineState && shownGraph === graph ? engineState : null;
  const engineRef = useRef<Engine | null>(null);
  const narrowRef = useRef(narrow);
  narrowRef.current = narrow;
  engineRef.current = engine;
  const canvas = useRef<HTMLCanvasElement>(null);

  const [{ colored, hidden }, setLegend] = useState(loadLegend);
  const [typeOn, setTypeOn] = useState<boolean[]>([]);
  // Navigation: the texts visited, last = open. Kept in the browser history, so the
  // back button (the browser's or the panel's) goes back one text.
  const [trail, setTrail] = useState<string[]>(() => {
    const uid = decodeURIComponent(location.hash.slice(1));
    const start = (history.state?.trail as string[] | undefined) ?? (uid ? [uid] : []);
    history.replaceState({ trail: start }, "");
    return start;
  });
  const trailRef = useRef(trail);
  trailRef.current = trail;
  useEffect(() => {
    const onPop = (e: PopStateEvent) => {
      const uid = decodeURIComponent(location.hash.slice(1));
      setTrail((e.state?.trail as string[] | undefined) ?? (uid ? [uid] : []));
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);
  const navigate = useCallback((next: string[]) => {
    setTrail(next);
    const url = new URLSearchParams(location.search);
    history.pushState({ trail: next }, "", `?${url}${next.length ? `#${next[next.length - 1]}` : ""}`);
  }, []);
  const [hover, setHover] = useState<{ i: number; x: number; y: number }>({ i: -1, x: 0, y: 0 });
  const [linksVersion, setLinksVersion] = useState(0);

  // Vectorial space
  const [threshold, setThreshold] = useState(0); // applied
  const [sliderValue, setSliderValue] = useState(0); // shown while dragging
  const [hideAlone, setHideAlone] = useState(true);
  const laidOutAt = useRef(0); // threshold the current positions were computed for

  // Search
  const input = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);

  // ---------- Load the view ----------
  useEffect(() => {
    let live = true;
    setGraph(null);
    setPairs(null);
    loadGraph(mapName).then(async (g) => {
      if (!live) return;
      if (g.sim) {
        const p = await loadPairs(g.sim);
        if (!live) return;
        const fromUrl = Number(new URLSearchParams(location.search).get("th"));
        const th = fromUrl >= g.sim.min && fromUrl <= g.sim.max ? fromUrl : g.sim.default;
        laidOutAt.current = g.sim.default; // positions in the JSON are for the default threshold
        setThreshold(th);
        setSliderValue(th);
        setPairs(p);
      }
      setTypeOn(g.types.map(() => true));
      setGraph(g);
    });
    return () => { live = false; };
  }, [mapName]);

  // ---------- Engine lifecycle ----------
  // The Rust engine is made once and given each view's graph (setGraph: same GPU setup, the
  // camera eases to the new points); the three.js one is made again for each view.
  const liveEngine = useRef<Engine | null>(null);
  useEffect(() => () => { liveEngine.current?.dispose(); liveEngine.current = null; }, []);
  useEffect(() => {
    if (!graph || !canvas.current) return;
    const cur = liveEngine.current;
    if (cur instanceof RustMapEngine) {
      cur.setGraph(graph, mapTheme(loadTheme(), graph));
      setShownGraph(graph);
      return;
    }
    let live = true;
    let made: Engine | null = null;
    void createEngine(canvas.current, graph, mapTheme(loadTheme(), graph), {
      onSelect: (i) => {
        chooseRef.current(i);
        if (narrowRef.current) setLegendOpen(false);
      },
      onHover: (i, x, y) => setHover({ i, x, y }),
    }).then((e) => {
      made = e;
      if (!live) return e.dispose();
      liveEngine.current = e;
      setEngine(e);
      setShownGraph(graph);
    });
    return () => {
      live = false;
      if (made instanceof MapEngine) {
        made.dispose();
        liveEngine.current = null;
        setEngine(null);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graph]);

  // Open a text or close (-1). Following a link (the text is a neighbour of the open one) adds it
  // to the trail; any other text (another point, a search result) starts a new trail.
  const choose = useCallback((i: number) => {
    const cur = trailRef.current;
    if (i < 0) { if (cur.length) navigate([]); return; }
    const uid = graph?.nodes[i]?.[0];
    if (!uid) return;
    setPanelOpen(true);
    if (cur[cur.length - 1] === uid) return;
    const open = graph!.nodes.findIndex((n) => n[0] === cur[cur.length - 1]);
    const linked = open >= 0 && !!engineRef.current?.adj[open]?.some(([j]) => j === i);
    navigate(linked ? [...cur, uid].slice(-30) : [uid]);
  }, [graph, navigate]);
  const chooseRef = useRef(choose);
  chooseRef.current = choose;

  const indexOf = useMemo(() => new Map(graph?.nodes.map((n, i) => [n[0], i]) ?? []), [graph]);

  // ---------- Push state into the engine ----------
  useEffect(() => {
    if (!engine || !graph) return;
    engine.setColored(graph.langs.map((l) => colored.has(l.iso)));
    engine.setVisible(graph.langs.map((l) => !hidden.has(l.iso)));
    try { localStorage.setItem(LEGEND_KEY, JSON.stringify({ colored: [...colored], hidden: [...hidden] })); } catch { /* private mode */ }
  }, [engine, graph, colored, hidden]);

  useEffect(() => { engine?.setTypes(typeOn); }, [engine, typeOn]);

  const available = useMemo(
    () => (graph && langTitles ? graph.nodes.map((n) => n[0] in langTitles) : null),
    [graph, langTitles],
  );
  useEffect(() => { engine?.setAvailable(available); }, [engine, available]);

  // The open text: last of the trail, if it is on this map and shown.
  const selected = useMemo(() => {
    const i = indexOf.get(trail[trail.length - 1] ?? "");
    return i !== undefined && (!available || available[i]) ? i : -1;
  }, [indexOf, trail, available]);
  useEffect(() => { engine?.select(selected, selected >= 0); }, [engine, selected]);
  // Closing the panel keeps the text selected on the map; a click on the empty map (or Escape)
  // deselects, a click on another text selects it and reopens the panel.
  const [panelOpen, setPanelOpen] = useState(true);
  useEffect(() => { if (selected >= 0) setPanelOpen(true); }, [selected]);
  // A text's reference: acronym, Pāli title, and the title in the texts' language (English
  // when the texts are in Pāli).
  const refOf = useCallback(
    (i: number): RefData => {
      const n = graph!.nodes[i]!;
      return { acronym: n[1], pali: n[3], title: translations?.[glossLang]?.titles[n[0]] || n[2] };
    },
    [graph, translations, glossLang],
  );
  const blurbOf = useCallback((i: number) => {
    const uid = graph!.nodes[i]![0];
    return blurbs.lang[uid] ?? blurbs.en[uid];
  }, [graph, blurbs]);
  const translatorOf = useCallback((i: number) => {
    const tr = translations?.[glossLang];
    const k = tr?.by[graph!.nodes[i]![0]];
    return k === undefined ? undefined : tr!.translators[k];
  }, [graph, translations, glossLang]);

  const colors = useMemo(() => (graph ? mapTheme(theme, graph) : null), [graph, theme]);
  useEffect(() => { if (engine && colors) engine.setTheme(colors); }, [engine, colors]);

  useEffect(() => {
    if (!engine || !graph?.sim || !pairs) return;
    const { edges, sims } = linksAt(pairs, threshold, graph.nodes.length);
    engine.setLinks(edges, sims, threshold !== laidOutAt.current);
    laidOutAt.current = threshold;
    setLinksVersion((v) => v + 1);
  }, [engine, graph, pairs, threshold]);

  useEffect(() => {
    if (engine && graph?.sim) engine.setHideAlone(hideAlone);
  }, [engine, graph, hideAlone, linksVersion]);

  // Debounce the slider: the layout re-runs when it settles.
  useEffect(() => {
    if (!graph?.sim || sliderValue === threshold) return;
    const timer = setTimeout(() => {
      setThreshold(sliderValue);
      const url = new URLSearchParams(location.search);
      url.set("th", String(sliderValue));
      history.replaceState(history.state, "", `?${url}${location.hash}`);
    }, 200);
    return () => clearTimeout(timer);
  }, [graph, sliderValue, threshold]);

  // ---------- Keyboard ----------
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && document.activeElement !== input.current) choose(-1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [choose]);

  // ---------- Derived ----------
  const linkCounts = useMemo(() => {
    const c = graph ? graph.types.map(() => 0) : [];
    if (graph) for (let i = 2; i < graph.edges.length; i += 3) c[graph.edges[i]!]!++;
    return c;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graph, linksVersion]);
  const alone = useMemo(() => (engine ? engine.adj.filter((a) => a.length === 0).length : 0),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [engine, linksVersion]);

  const searchIndex = useMemo(() => graph?.nodes.map((n) => ({ uid: normalize(n[0]), acr: normalize(n[1]), text: normalize(`${n[2]} ${n[3]} ${langTitles?.[n[0]] ?? ""}`) })) ?? [], [graph, langTitles]);
  const hits = useMemo(() => {
    const nq = normalize(query);
    if (nq.length < 2 || !engine) return [];
    const scored: [number, number][] = [];
    searchIndex.forEach((s, i) => {
      if (available && !available[i]) return;
      let score = -1;
      if (s.uid === nq || s.acr === nq) score = 3;
      else if (s.uid.startsWith(nq) || s.acr.startsWith(nq)) score = 2;
      else if (nq.length >= 3 && s.text.includes(nq)) score = 1;
      if (score >= 0) scored.push([i, score * 1000 + engine.adj[i]!.length]);
    });
    return scored.sort((a, b) => b[1] - a[1]).slice(0, 30).map(([i]) => i);
  }, [query, searchIndex, engine, available]);
  useEffect(() => setActive(0), [hits]);

  const pick = (i: number) => {
    choose(i);
    setQuery("");
    input.current?.blur();
  };

  const toggle = (set: Set<string>, iso: string) => {
    const next = new Set(set);
    if (next.has(iso)) next.delete(iso); else next.add(iso);
    return next;
  };


  // The texts' language, next to the view chip.
  const textsMenu = (
      <Menu
        name={t("texts.name")}
        label={TEXT_LANGS.find((l) => l.value === textLang)!.label}
        value={textLang}
        options={TEXT_LANGS.map(({ value, label }) => ({
          value,
          label,
          hint: value === "pli"
            ? t("texts.all")
            : t("texts.translated", { count: Object.keys(translations?.[value]?.titles ?? {}).length }),
        }))}
        onChange={(v) => setTextLangChoice(v)}
      />
  );

  return (
    <div className="flex h-dvh flex-col">
      {/* Header: palicanon's results header (palisearch/web/src/App.tsx). */}
      <header className="relative z-40 border-b border-line">
        <div className="mx-auto grid w-full max-w-[64rem] grid-cols-[1fr_auto] items-center gap-x-6 gap-y-3 px-6 py-4 sm:grid-cols-[11rem_minmax(0,40rem)_1fr]">
          <div className="flex items-center gap-2">
            <a href="./" className="flex shrink-0 items-center gap-2 font-serif text-lg font-semibold text-heading">
              <img src="/lotus.png" alt="" width={22} height={22} />
              {t("brand")}
            </a>
          </div>
          <div
            className="relative col-span-2 min-w-0 sm:col-span-1"
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") { e.preventDefault(); setActive((a) => Math.min(a + 1, hits.length - 1)); }
              else if (e.key === "ArrowUp") { e.preventDefault(); setActive((a) => Math.max(a - 1, 0)); }
              else if (e.key === "Escape") { setQuery(""); input.current?.blur(); }
            }}
          >
            <SearchBar ref={input} value={query} large={false} busy={false} onChange={setQuery} onSubmit={() => hits[active] !== undefined && pick(hits[active]!)} />
            {hits.length > 0 && graph && (
              <ul className={`absolute inset-x-0 z-20 mt-2 max-h-[50vh] overflow-auto p-1 ${card}`}>
                {hits.map((i, k) => (
                  <TextRow key={i} data={refOf(i)} color={colors?.groups[graph.nodes[i]![5]]} active={k === active} onPointerEnter={() => setActive(k)} onClick={() => pick(i)} />
                ))}
              </ul>
            )}
          </div>
          <div className="col-start-2 row-start-1 flex items-center gap-1 justify-self-end sm:col-start-3">
            <LangSwitch />
            <button
              type="button"
              onClick={() => setSettingsOpen(true)}
              aria-label={t("settings.title")}
              title={t("settings.title")}
              className="grid size-9 cursor-pointer place-items-center rounded-full text-muted transition hover:bg-chip hover:text-heading"
            >
              <GearIcon />
            </button>
          </div>
        </div>
      </header>

      {/* Body: palicanon's chat layout — docked side columns, full height, separated by lines. */}
      <div className="relative flex min-h-0 flex-1">
        {graph && colors && legendOpen && narrow && (
          <button type="button" aria-label={t("legend.toggle")} onClick={() => setLegendOpen(false)} className="absolute inset-0 z-20 cursor-default bg-black/40" />
        )}
        {graph && colors && legendOpen && (
          <aside className="absolute inset-y-0 left-0 z-30 flex w-64 max-w-[85vw] shrink-0 flex-col border-r border-line bg-page md:static">
            <div className="min-h-0 flex-1 overflow-y-auto">
            <Legend
              graph={graph}
              theme={colors}
              available={available}
              colored={colored}
              hidden={hidden}
              typeOn={typeOn}
              linkCounts={linkCounts}
              onColor={(iso) => setLegend((s) => ({ ...s, colored: toggle(s.colored, iso) }))}
              onHide={(iso) => setLegend((s) => ({ ...s, hidden: toggle(s.hidden, iso) }))}
              onType={(t) => setTypeOn((on) => on.map((v, k) => (k === t ? !v : v)))}
            >
              {graph.sim && (
                <SimControls
                  min={graph.sim.min}
                  max={graph.sim.max}
                  value={sliderValue}
                  links={linkCounts[0] ?? 0}
                  alone={alone}
                  hideAlone={hideAlone}
                  onChange={setSliderValue}
                  onHideAlone={setHideAlone}
                />
              )}
            </Legend>
            </div>
          </aside>
        )}

        <main className="relative min-w-0 flex-1 overflow-hidden">
          <canvas ref={canvas} className="absolute inset-0 block size-full" />
          {/* View chips, centred over the map like palicanon's filter chips under the search. */}
          <div className="pointer-events-none absolute inset-x-0 top-0 z-10 flex justify-center px-4 pt-3">
            <div className="pointer-events-auto flex flex-wrap items-center justify-center gap-2">
              {/* Collections panel toggle, left of the view chips. */}
              <button
                type="button"
                onClick={() => setLegendOpen((o) => !o)}
                aria-label={t("legend.toggle")}
                aria-pressed={legendOpen}
                title={t("legend.toggle")}
                className={`grid size-[30px] shrink-0 cursor-pointer place-items-center rounded-full border transition hover:text-heading ${legendOpen ? "border-gold/60 text-heading" : "border-line text-muted hover:border-card-edge"}`}
              >
                <PanelIcon />
              </button>
              <Menu
                name={t("views.name")}
                label={t(`views.label_${mapName}`)}
                value={mapName}
                options={MAPS.map(({ value }) => ({ value, label: t(`views.label_${value}`), hint: t(`views.${value}`) }))}
                onChange={(v) => {
                  const url = new URLSearchParams(location.search);
                  url.set("map", v);
                  url.delete("th");
                  history.replaceState(history.state, "", `?${url}${location.hash}`);
                  setMapName(v);
                }}
              />
              {textsMenu}
            </div>
          </div>
          {!graph && (
            <p className="absolute inset-0 grid place-items-center font-serif text-xl italic text-muted">{t("loading")}</p>
          )}
        </main>

        {graph && engine && colors && selected >= 0 && panelOpen && (
          <aside className="absolute inset-y-0 right-0 z-30 w-full shrink-0 border-l border-line bg-page sm:w-80 lg:static">
            <TextPanel
              key={`${selected}-${linksVersion}`}
              graph={graph}
              theme={colors}
              index={selected}
              neighbours={(engine.adj[selected] ?? []).filter(([j]) => !available || available[j])}
              simOf={(a, b) => engine.simOf(a, b)}
              refOf={refOf}
              blurbOf={blurbOf}
              translatorOf={translatorOf}
              dpd={mapName === "sim"}
              onPick={(j) => choose(j)}
              onClose={() => setPanelOpen(false)}
              trail={trail.map((uid) => {
                const k = indexOf.get(uid);
                return { uid, label: k !== undefined ? graph.nodes[k]![1] : uid };
              })}
              onJump={(k) => navigate(trail.slice(0, k + 1))}
            />
          </aside>
        )}
      </div>

      {/* palicanon.app's footer; the App Store / Instagram / Ko-fi icons under the legend column. */}
      <Footer social={false} leading={<SocialLinks size={22} gap="gap-4" campaign="PaliWebMap" />} />

      {graph && hover.i >= 0 && (
        <div
          className={`pointer-events-none fixed z-40 max-w-[280px] px-2.5 py-1.5 text-[13px] ${card}`}
          style={{ left: hover.x + 12, top: hover.y + 12 }}
        >
          <SuttaRef {...refOf(hover.i)} />
          <span className="block text-subtle">{t("tooltip.links", { count: engine?.adj[hover.i]?.length ?? 0 })}</span>
        </div>
      )}
      {settingsOpen && <SettingsPanel onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
