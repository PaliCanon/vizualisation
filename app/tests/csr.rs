//! The Map fragment in a real browser (task 012), mounted the way the PaliCanon app mounts
//! it: `palicanon_ui_lib::csr::mount`, `MapRoutes` inside the host's `ParentRoute /map`, a
//! `MapData` serving the committed `public/` files (a fake that can hold a file back). The
//! host shows and hides the routes as the app shows and hides the map's tab: hiding disposes
//! the fragment's owner, so its cleanups run.
//!
//! Run with `wasm-bindgen-test-runner` and a headless Chrome (as ui-lib's tests/csr.rs):
//! `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner
//! CHROMEDRIVER=<chromedriver> WASM_BINDGEN_TEST_TIMEOUT=600 cargo test -p vizualisation
//! --no-default-features --features csr --target wasm32-unknown-unknown --test csr`.
//!
//! Headless Chrome draws with WebGL2 (SwiftShader): these mounts test the GL release
//! (`WEBGL_lose_context`). WebGPU's `device.destroy()` is not reached here.
//!
//! `csr::mount` leaks its owner (the app lives as long as the page): the file mounts one
//! host, once, and runs its steps in one test, in order.

#![cfg(all(target_arch = "wasm32", feature = "csr"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures::channel::oneshot::{self, Sender};
use leptos::prelude::*;
use leptos_router::components::{Outlet, ParentRoute, Router, Routes};
use leptos_router::path;
use palicanon_ui_lib::csr;
use vizualisation::fragment::{MapData, MapFile, MapHead, MapProvider, MapRoutes};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

// Counts the event listeners and resize observers the page holds, from now on: a listener
// added and not removed, an observer observing and not disconnected.
#[wasm_bindgen(inline_js = r"
let installed = false;
const listeners = [];
const observers = new Set();
const captureOf = (options) =>
  typeof options === 'boolean' ? options : !!(options && options.capture);
export function count_from_now() {
  if (installed) return;
  installed = true;
  const add = EventTarget.prototype.addEventListener;
  const remove = EventTarget.prototype.removeEventListener;
  EventTarget.prototype.addEventListener = function (type, listener, options) {
    const capture = captureOf(options);
    const known = listeners.some(
      (l) => l.target === this && l.type === type && l.listener === listener && l.capture === capture,
    );
    if (!known && listener) listeners.push({ target: this, type, listener, capture });
    return add.call(this, type, listener, options);
  };
  EventTarget.prototype.removeEventListener = function (type, listener, options) {
    const capture = captureOf(options);
    const at = listeners.findIndex(
      (l) => l.target === this && l.type === type && l.listener === listener && l.capture === capture,
    );
    if (at >= 0) listeners.splice(at, 1);
    return remove.call(this, type, listener, options);
  };
  const observe = ResizeObserver.prototype.observe;
  const disconnect = ResizeObserver.prototype.disconnect;
  ResizeObserver.prototype.observe = function (target, options) {
    observers.add(this);
    return observe.call(this, target, options);
  };
  ResizeObserver.prototype.disconnect = function () {
    observers.delete(this);
    return disconnect.call(this);
  };
}
export function live_listeners() { return listeners.length; }
export function live_listener_kinds() {
  return listeners.map((l) => (l.target === window ? 'window:' : l.target === document ? 'document:' : (l.target.tagName || l.target.constructor.name) + ':') + l.type).join(' ');
}
export function live_observers() { return observers.size; }
")]
extern "C" {
    fn count_from_now();
    fn live_listeners() -> u32;
    fn live_listener_kinds() -> String;
    fn live_observers() -> u32;
}

/// The committed data files, as the app bundles them.
fn bundled(name: &str) -> Option<Vec<u8>> {
    let bytes: &[u8] = match name {
        "graph.json" => include_bytes!("../../public/graph.json"),
        "graph-sim.json" => include_bytes!("../../public/graph-sim.json"),
        "graph-sim.bin" => include_bytes!("../../public/graph-sim.bin"),
        "translations.json" => include_bytes!("../../public/translations.json"),
        "blurbs-en.json" => include_bytes!("../../public/blurbs-en.json"),
        "blurbs-fr.json" => include_bytes!("../../public/blurbs-fr.json"),
        _ => return None,
    };
    Some(bytes.to_vec())
}

/// The senders of the asks a fake holds back.
type Held = Arc<Mutex<Vec<Sender<Option<Vec<u8>>>>>>;

/// A `MapData` over the committed files that records what it is asked, can hold one file
/// back until the test releases it, and can answer `None` for one. Clones share all of it.
#[derive(Clone, Default)]
struct Fake {
    asked: Arc<Mutex<Vec<&'static str>>>,
    hold: Arc<Mutex<Option<&'static str>>>,
    held: Held,
    absent: Arc<Mutex<Option<&'static str>>>,
}

impl Fake {
    /// How many times `name` was asked.
    fn asked(&self, name: &str) -> usize {
        self.asked
            .lock()
            .expect("asked")
            .iter()
            .filter(|n| **n == name)
            .count()
    }

    /// How many asks, of every file.
    fn total(&self) -> usize {
        self.asked.lock().expect("asked").len()
    }

    /// Answers `None` to every later ask of `name` (as a missing file); `None` serves all.
    fn absent(&self, name: Option<&'static str>) {
        *self.absent.lock().expect("absent") = name;
    }

    /// Holds every later ask of `name` until [`Fake::release`]; `None` answers at once again.
    fn hold(&self, name: Option<&'static str>) {
        *self.hold.lock().expect("hold") = name;
    }

    /// Answers the held asks with the file; how many were still awaited.
    fn release(&self, name: &str) -> usize {
        self.held
            .lock()
            .expect("held")
            .drain(..)
            .filter(|tx| !tx.is_canceled())
            .map(|tx| tx.send(bundled(name)).is_ok())
            .filter(|sent| *sent)
            .count()
    }
}

impl MapData for Fake {
    fn file(&self, name: &'static str) -> MapFile {
        self.asked.lock().expect("asked").push(name);
        if *self.absent.lock().expect("absent") == Some(name) {
            return Box::pin(async { None });
        }
        if *self.hold.lock().expect("hold") == Some(name) {
            let (tx, rx) = oneshot::channel();
            self.held.lock().expect("held").push(tx);
            return Box::pin(async move { rx.await.ok().flatten() });
        }
        let bytes = bundled(name);
        Box::pin(async move { bytes })
    }
}

/// The app's host: the language, a router, the map's provider (no head of its own), and the
/// map's routes under `/map` while `shown`.
#[component]
fn Host(data: Arc<dyn MapData>, shown: ArcRwSignal<bool>) -> impl IntoView {
    palicanon_ui_lib::provide_language();
    view! {
        <Router>
            <MapProvider data=data head=|_: MapHead| ()>
                <div id="host">
                    {move || {
                        shown
                            .get()
                            .then(|| {
                                view! {
                                    <Routes fallback=|| view! { <p id="host-404">"host 404"</p> }>
                                        <ParentRoute path=path!("/map") view=Outlet>
                                            <MapRoutes />
                                        </ParentRoute>
                                    </Routes>
                                }
                            })
                    }}
                </div>
            </MapProvider>
        </Router>
    }
}

/// A panic anywhere in the page since the hook was set.
static PANICKED: AtomicBool = AtomicBool::new(false);

/// Records a panic in [`PANICKED`], then logs it.
fn record_panics() {
    std::panic::set_hook(Box::new(|info| {
        PANICKED.store(true, Ordering::SeqCst);
        web_sys::console::error_1(&format!("panic: {info}").into());
    }));
}

/// Gives the map's canvas a box (the page has no Tailwind: the classes the map relies on
/// for its canvas, by hand) and a desktop-wide main.
fn style_the_page() {
    let style = document().create_element("style").expect("a style");
    style.set_text_content(Some(
        ".absolute{position:absolute}.relative{position:relative}.inset-0{inset:0}\
         .block{display:block}.size-full{width:100%;height:100%}\
         main{display:block;position:relative;width:800px;height:600px}",
    ));
    document()
        .head()
        .expect("a head")
        .append_child(&style)
        .expect("styled");
}

/// Puts the page at `url` and tells the router (`popstate`, as back and forward do).
fn visit(url: &str) {
    window()
        .history()
        .expect("history")
        .replace_state_with_url(&JsValue::NULL, "", Some(url))
        .expect("same origin");
    let event = web_sys::PopStateEvent::new("popstate").expect("an event");
    window().dispatch_event(&event).expect("dispatched");
}

/// The page's path and query.
fn location() -> String {
    let l = window().location();
    format!(
        "{}{}",
        l.pathname().unwrap_or_default(),
        l.search().unwrap_or_default()
    )
}

/// The host's text.
fn text() -> String {
    document()
        .get_element_by_id("host")
        .and_then(|el| el.text_content())
        .unwrap_or_default()
}

/// `<html data-engine>`, the graphics API drawing.
fn data_engine() -> Option<String> {
    document()
        .document_element()
        .and_then(|html| html.get_attribute("data-engine"))
}

/// The bytes of the wasm module's linear memory.
fn memory_bytes() -> u32 {
    wasm_bindgen::memory()
        .unchecked_into::<js_sys::WebAssembly::Memory>()
        .buffer()
        .unchecked_into::<js_sys::ArrayBuffer>()
        .byte_length()
}

/// `document.cookie`.
fn cookies() -> String {
    document()
        .unchecked_into::<web_sys::HtmlDocument>()
        .cookie()
        .unwrap_or_default()
}

/// `localStorage[key]`.
fn stored(key: &str) -> Option<String> {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(key).ok().flatten())
}

/// `localStorage[key] = value`, or the entry removed.
fn store(key: &str, value: Option<&str>) {
    let storage = window().local_storage().ok().flatten().expect("a storage");
    match value {
        Some(value) => storage.set_item(key, value).expect("stored"),
        None => storage.remove_item(key).expect("removed"),
    }
}

/// The map's `MapError`, over the map: its data did not arrive.
fn map_error() -> bool {
    text().contains("The map could not load")
}

/// The element `selector` finds in the host, as an `HtmlElement`; `None` when there is none.
fn host_element(selector: &str) -> Option<web_sys::HtmlElement> {
    document()
        .query_selector(&format!("#host {selector}"))
        .ok()
        .flatten()
        .map(JsCast::unchecked_into)
}

/// Waits `ms` milliseconds (the page's timers and frames run meanwhile).
async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .expect("a timer");
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("slept");
}

/// Waits until `done`, at most 60 s; panics naming `what` otherwise.
async fn until(what: &str, done: impl Fn() -> bool) {
    for _ in 0..3000 {
        if done() {
            return;
        }
        sleep(20).await;
    }
    panic!("timed out waiting for {what}; the host says: {}", text());
}

/// A click on the canvas at `(x, y)`, its top left's CSS pixels: down, then up in place.
#[allow(clippy::cast_possible_truncation, reason = "CSS pixels fit i32")]
fn click_canvas(x: f64, y: f64) {
    let canvas = document()
        .query_selector("#host canvas")
        .ok()
        .flatten()
        .expect("the canvas");
    let rect = canvas.get_bounding_client_rect();
    for kind in ["pointerdown", "pointerup"] {
        let init = web_sys::PointerEventInit::new();
        init.set_pointer_id(1);
        init.set_pointer_type("mouse");
        init.set_bubbles(true);
        init.set_client_x((rect.left() + x) as i32);
        init.set_client_y((rect.top() + y) as i32);
        let event = web_sys::PointerEvent::new_with_event_init_dict(kind, &init).expect("an event");
        canvas.dispatch_event(&event).expect("dispatched");
    }
}

/// Clicks the canvas on a grid until a text is selected (the address goes to `/map/<uid>`);
/// whether one was.
fn select_by_click() -> bool {
    for y in (10..150).step_by(4) {
        for x in (10..800).step_by(4) {
            click_canvas(f64::from(x), f64::from(y));
            if location().starts_with("/map/") {
                return true;
            }
        }
    }
    false
}

/// Mounts the map at `url` and waits until it draws (one live engine view).
async fn mount_at(shown: &ArcRwSignal<bool>, url: &str) {
    visit(url);
    shown.set(true);
    until("the map to draw", || {
        data_engine().is_some() && map_engine::live_views() == 1
    })
    .await;
}

/// Unmounts the map and lets a frame or two pass.
async fn unmount(shown: &ArcRwSignal<bool>) {
    shown.set(false);
    sleep(50).await;
}

#[wasm_bindgen_test]
#[allow(
    clippy::too_many_lines,
    reason = "one host per page (csr::mount leaks it): the steps run in order"
)]
async fn the_map_mounts_and_unmounts_under_map() {
    record_panics();
    style_the_page();
    if let Ok(Some(storage)) = window().local_storage() {
        storage.clear().expect("cleared");
        storage.set_item("pc_lang", "en").expect("stored");
    }
    count_from_now();
    visit("/map");
    let fake = Fake::default();
    let shown = ArcRwSignal::new(false);
    {
        let data: Arc<dyn MapData> = Arc::new(fake.clone());
        let shown = shown.clone();
        csr::mount(move || view! { <Host data=data shown=shown /> });
    }
    sleep(50).await;
    let listeners_before = live_listeners();

    // ---------- Drawn, a click selects, a text's panel, the 404 ----------
    mount_at(&shown, "/map").await;
    assert_eq!(location(), "/map");
    web_sys::console::log_1(&format!("measured: drawn with {:?}", data_engine()).into());
    assert!(
        select_by_click(),
        "a click on the map selects a text: {}",
        location()
    );
    until("the clicked text's panel", || {
        text().contains("Read on SuttaCentral")
    })
    .await;
    visit("/map/mn10");
    until("MN 10's panel", || text().contains("Satipaṭṭhāna")).await;
    assert_eq!(location(), "/map/mn10");
    visit("/map/zz999");
    until("the 404", || text().contains("Anattā")).await;
    let home = document()
        .query_selector("#host a")
        .ok()
        .flatten()
        .and_then(|a| a.get_attribute("href"));
    assert_eq!(home.as_deref(), Some("/map"), "the 404's home link");
    visit("/map");
    until("the map again", || text().contains("SuttaCentral")).await;

    // ---------- Unmounted: nothing left ----------
    unmount(&shown).await;
    assert_eq!(map_engine::live_views(), 0, "the engine view is dropped");
    assert_eq!(data_engine(), None, "data-engine is removed");
    assert!(!PANICKED.load(Ordering::SeqCst), "no panic");

    // ---------- Unmounted while a file is held ----------
    fake.hold(Some("graph.json"));
    let before = fake.asked("graph.json");
    visit("/map");
    shown.set(true);
    until("graph.json asked", || fake.asked("graph.json") > before).await;
    let asked = fake.asked("graph.json");
    unmount(&shown).await;
    fake.hold(None);
    let awaited = fake.release("graph.json");
    sleep(100).await;
    assert!(
        !PANICKED.load(Ordering::SeqCst),
        "an unmount during a load panicked"
    );
    assert_eq!(awaited, 0, "nobody awaits the held file once unmounted");
    assert_eq!(fake.asked("graph.json"), asked, "not asked again");
    assert_eq!(map_engine::live_views(), 0);
    assert_eq!(data_engine(), None);

    // ---------- Unmounted while each file is held: never asked again ----------
    for (name, url, texts) in [
        ("graph.json", "/map", None),
        ("graph-sim.json", "/map/sim", None),
        ("graph-sim.bin", "/map/sim", None),
        ("translations.json", "/map", None),
        ("blurbs-en.json", "/map", None),
        ("blurbs-fr.json", "/map", Some("fr|en")),
    ] {
        store("viz_textlang", texts);
        fake.hold(Some(name));
        let before = fake.asked(name);
        visit(url);
        shown.set(true);
        until(&format!("{name} asked"), || fake.asked(name) > before).await;
        sleep(100).await;
        unmount(&shown).await;
        let total = fake.total();
        fake.hold(None);
        let awaited = fake.release(name);
        sleep(200).await;
        assert!(
            !PANICKED.load(Ordering::SeqCst),
            "{name}: an unmount during its load panicked"
        );
        assert_eq!(awaited, 0, "{name}: nobody awaits it once unmounted");
        assert_eq!(
            fake.total(),
            total,
            "{name}: no file asked after the unmount"
        );
        assert_eq!(map_engine::live_views(), 0, "{name}");
        assert_eq!(data_engine(), None, "{name}");
    }
    store("viz_textlang", None);

    // ---------- A file absent: the map says so; a language's summaries absent: none ----------
    for (name, url) in [
        ("graph.json", "/map"),
        ("graph-sim.json", "/map/sim"),
        ("graph-sim.bin", "/map/sim"),
        ("translations.json", "/map"),
        ("blurbs-en.json", "/map"),
    ] {
        fake.absent(Some(name));
        let ahead = fake.asked("graph-sim.bin");
        visit(url);
        shown.set(true);
        until(&format!("MapError without {name}"), map_error).await;
        if url == "/map" && name != "graph.json" {
            // The view's data in (the vectorial space's asked ahead after it): still said.
            until("the view installed", || fake.asked("graph-sim.bin") > ahead).await;
            sleep(100).await;
            assert!(map_error(), "{name}: MapError stays once the view is in");
        }
        unmount(&shown).await;
        fake.absent(None);
        assert_eq!(map_engine::live_views(), 0, "{name}");
        assert!(!PANICKED.load(Ordering::SeqCst), "{name}: a panic");
    }
    store("viz_textlang", Some("fr|en"));
    fake.absent(Some("blurbs-fr.json"));
    let before = fake.asked("blurbs-fr.json");
    mount_at(&shown, "/map/mn10").await;
    until("MN 10's English summary", || {
        text().contains("Here the Buddha details the seventh factor")
    })
    .await;
    assert!(
        fake.asked("blurbs-fr.json") > before,
        "the French summaries asked"
    );
    assert!(!text().contains("Le Bouddha détaille"), "{}", text());
    assert!(!map_error(), "no MapError without blurbs-fr.json");
    unmount(&shown).await;
    fake.absent(None);
    store("viz_textlang", None);

    // ---------- 20 mount-unmount cycles ----------
    let mut after_unmount = Vec::new();
    let mut memory = Vec::new();
    for cycle in 1..=20 {
        mount_at(&shown, "/map").await;
        assert_eq!(map_engine::live_views(), 1, "cycle {cycle}: one view");
        if cycle == 20 {
            assert!(select_by_click(), "the 20th mount: a click selects");
        }
        unmount(&shown).await;
        assert_eq!(map_engine::live_views(), 0, "cycle {cycle}: no view");
        assert_eq!(data_engine(), None, "cycle {cycle}: data-engine");
        assert_eq!(live_observers(), 0, "cycle {cycle}: a resize observer left");
        after_unmount.push(live_listeners());
        memory.push(memory_bytes());
        assert!(!PANICKED.load(Ordering::SeqCst), "cycle {cycle}: a panic");
    }
    web_sys::console::log_1(
        &format!(
            "measured: listeners before the map {listeners_before}, after each unmount \
             {after_unmount:?}; wasm memory after each unmount {memory:?}"
        )
        .into(),
    );
    assert!(
        after_unmount.iter().all(|n| *n == listeners_before),
        "the listeners left after each unmount: {after_unmount:?}, before the map: \
         {listeners_before}: {}",
        live_listener_kinds()
    );
    let (fourth, last) = (memory[3], memory[19]);
    assert!(
        last.abs_diff(fourth) <= 1 << 20,
        "wasm memory after the 4th unmount {fourth}, after the 20th {last}: {memory:?}"
    );

    // ---------- The preferences in localStorage, no cookie, read back on a mount ----------
    mount_at(&shown, "/map").await;
    host_element(r#"button[aria-label="Hide Majjhima Nikāya"]"#)
        .expect("MN's eye")
        .click();
    until("the hidden collection stored", || {
        stored("viz_legend").is_some_and(|v| v.split('|').nth(1) == Some("mn"))
    })
    .await;
    host_element(r#"button[aria-controls="viz-texts"]"#)
        .expect("the texts chip")
        .click();
    host_element("#viz-texts-2").expect("Français").click();
    until("the texts language stored", || {
        stored("viz_textlang").as_deref() == Some("fr|en")
    })
    .await;
    assert!(!cookies().contains("viz_"), "no cookie: {}", cookies());
    let toggle = document()
        .query_selector(r#"#host button[aria-pressed][title="Show / hide the collections"]"#)
        .ok()
        .flatten()
        .expect("the legend's toggle")
        .unchecked_into::<web_sys::HtmlElement>();
    toggle.click();
    until("the legend stored closed", || {
        stored("viz_legend").is_some_and(|v| v.ends_with("|0"))
    })
    .await;
    assert_eq!(stored("viz_legend").as_deref(), Some("|mn|0"));
    assert!(!cookies().contains("viz_"), "no cookie: {}", cookies());

    unmount(&shown).await;
    mount_at(&shown, "/map").await;
    assert!(
        host_element(r#"button[aria-label="Show Majjhima Nikāya"]"#).is_some(),
        "MN read back hidden: {}",
        text()
    );
    assert_eq!(
        host_element("#viz-texts-2").and_then(|li| li.get_attribute("aria-selected")),
        Some("true".to_owned()),
        "Français read back"
    );
    assert_eq!(
        host_element(r#"button[aria-pressed][title="Show / hide the collections"]"#)
            .and_then(|b| b.get_attribute("aria-pressed")),
        Some("false".to_owned()),
        "the legend read back closed"
    );
    assert!(!cookies().contains("viz_"), "no cookie: {}", cookies());

    unmount(&shown).await;
    assert_eq!(map_engine::live_views(), 0);
}
