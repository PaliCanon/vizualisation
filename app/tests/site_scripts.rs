//! `scripts/smoke.sh` and `scripts/rollout-check.sh` as processes (task 005,
//! slice S5): against the real binary and against a fake server that plays
//! every way a deployment can be wrong. A check that passes only on the
//! happy path proves nothing, so each check has a server that must fail it,
//! and the failure names the check. The origin secret and the dev password
//! are in no output and no argument.
//!
//! From search-front/app/tests/site_scripts.rs. What differs: the checks of
//! this service (design v4): `/health` behind the origin guard only, the
//! password gate of dev (401 with `WWW-Authenticate` without the password,
//! the pages and the files behind it; prod has no `WWW-Authenticate`), the
//! boot that refuses an unset `VIZ_GATE` (exit 2). No API, so no live search
//! and no token shape: the `search` and `token` checks and the
//! `token-pattern` file are gone; `smoke.sh` checks instead that no
//! response and no log of the binary holds its throwaway secret or password
//! (`leak`), and runs the binary as prod does too (`prod`). The `hydrate`
//! check (search-front 017) drives a fake Chrome; in dev it must reach the
//! page through a launcher page, the password off Chrome's command line.
#![cfg(feature = "ssr")]

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// The secret the service and the scripts share in these tests.
const SECRET: &str = "origin_canary_scripts_c40e17";
/// The dev password the service and the scripts share in these tests.
const PASSWORD: &str = "gate_canary_scripts_9a1f03";

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../scripts")
        .join(name)
}

/// A free port, never handed out twice by this test binary.
fn free_port() -> u16 {
    static LAST: AtomicU16 = AtomicU16::new(0);
    loop {
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("a free port")
            .local_addr()
            .expect("an address")
            .port();
        if LAST.swap(port, Ordering::SeqCst) != port {
            return port;
        }
    }
}

/// A fresh temp path for this test binary.
fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("viz-site-scripts-{}-{name}", std::process::id()))
}

// ---------------------------------------------------------------- fake server

/// A request as the fake server sees it.
#[derive(Clone)]
struct Req {
    path: String,
    headers: Vec<(String, String)>,
}

impl Req {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Whether the request carries the right origin secret.
    fn authed(&self) -> bool {
        self.header("x-origin-auth") == Some(SECRET)
    }

    /// Whether the request gives the dev password (HTTP Basic, any user).
    fn admitted(&self) -> bool {
        self.header("authorization")
            .and_then(|value| value.strip_prefix("Basic "))
            .and_then(|encoded| STANDARD.decode(encoded.trim()).ok())
            .and_then(|pair| String::from_utf8(pair).ok())
            .is_some_and(|pair| pair.split_once(':').is_some_and(|(_, p)| p == PASSWORD))
    }

    /// The request as the service sees it behind the Worker (which adds
    /// the header).
    fn through_worker(&self) -> Req {
        let mut req = self.clone();
        req.headers.retain(|(n, _)| n != "x-origin-auth");
        req.headers.push(("x-origin-auth".into(), SECRET.into()));
        req
    }
}

/// A response the fake server sends.
struct Resp {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Resp {
    fn new(status: u16, body: &str) -> Self {
        Resp {
            status,
            headers: Vec::new(),
            body: body.to_owned(),
        }
    }

    fn with(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// A server on a local port answering with `handler`; it keeps every
/// request it received.
struct Fake {
    port: u16,
    seen: Arc<Mutex<Vec<Req>>>,
}

impl Fake {
    fn start(handler: impl Fn(&Req) -> Resp + Send + 'static) -> Fake {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free port");
        let port = listener.local_addr().expect("an address").port();
        let seen: Arc<Mutex<Vec<Req>>> = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&seen);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                answer(stream, &handler, &kept);
            }
        });
        Fake { port, seen }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// How many requests carried the secret, and how many the password.
    fn authed_and_admitted(&self) -> (usize, usize) {
        let seen = self.seen.lock().expect("lock");
        (
            seen.iter().filter(|r| r.authed()).count(),
            seen.iter().filter(|r| r.admitted()).count(),
        )
    }
}

fn answer(mut stream: TcpStream, handler: &impl Fn(&Req) -> Resp, seen: &Mutex<Vec<Req>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut raw = Vec::new();
    let mut buf = [0u8; 2048];
    while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
        }
    }
    let text = String::from_utf8_lossy(&raw).into_owned();
    let mut lines = text.lines();
    let path = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_owned();
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let req = Req { path, headers };
    let resp = handler(&req);
    seen.lock().expect("lock").push(req);
    let mut head = format!(
        "HTTP/1.1 {} X\r\ncontent-length: {}\r\nconnection: close\r\n",
        resp.status,
        resp.body.len()
    );
    for (n, v) in &resp.headers {
        let _ = write!(head, "{n}: {v}\r\n");
    }
    let _ = stream.write_all(format!("{head}\r\n{}", resp.body).as_bytes());
}

const WASM: &str = "/pkg/vizualisation.wasm";
/// The stylesheet the page links (a release build links it by its hashed
/// name, `/pkg/vizualisation.<hash>.css`; the scripts read whichever the
/// page names).
const CSS: &str = "/pkg/vizualisation.css";

/// The probe's body as the real service writes it: the release, then each
/// graph file's stamp (`app/src/data.rs`, `stamp_lines`).
const HEALTHZ: &str = "ok 63bdba3\ngraph.json: none\ngraph-sim.json: none\n";

/// The release the fakes serve by default and the gate is told to expect:
/// main at a commit (a tag has an empty sha).
const RELEASE_ENV: &str = "main";
const RELEASE_SHA: &str = "34c489c";

/// The two release metas as the SSR head writes them (task 016): the env
/// and the sha, the sha meta always there, `content=""` for a tag.
fn release_metas(env: &str, sha: &str) -> String {
    format!(
        "<meta name=\"viz-release-env\" content=\"{env}\">\
         <meta name=\"viz-release-sha\" content=\"{sha}\">"
    )
}

/// The page the real service renders, as far as the scripts read it.
fn page(extra: &str) -> String {
    page_with(&release_metas(RELEASE_ENV, RELEASE_SHA), extra)
}

/// The same page with the head's release `metas` as given (right after the
/// viewport meta, where the real head writes them).
fn page_with(metas: &str, extra: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><title>Suttas Map — PaliCanon</title>\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">{metas}\
         <link rel=\"modulepreload\" href=\"/pkg/vizualisation.js\">\
         <link rel=\"preload\" href=\"{WASM}\" as=\"fetch\" type=\"application/wasm\">\
         <link id=\"leptos\" rel=\"stylesheet\" href=\"{CSS}\">\
         </head><body>{extra}</body></html>"
    )
}

/// A healthy deployment, in the design's order: the origin guard on every
/// path (403, empty body), `/health` behind it alone, then the password
/// gate when `gated` (401 asking for it), then the page, the wasm and the
/// stylesheet.
fn serve(req: &Req, gated: bool) -> Resp {
    if !req.authed() {
        return Resp::new(403, "").with("cache-control", "no-store");
    }
    if req.path == "/health" {
        return Resp::new(200, HEALTHZ).with("content-type", "text/plain");
    }
    if gated && !req.admitted() {
        return Resp::new(401, "")
            .with("www-authenticate", "Basic realm=\"PaliCanon viz dev\"")
            .with("cache-control", "private, no-store");
    }
    if req.path == WASM {
        return Resp::new(200, "\0asm").with("content-type", "application/wasm");
    }
    if req.path == CSS {
        return Resp::new(200, "body{}").with("content-type", "text/css");
    }
    if req.path == "/" {
        return Resp::new(200, &page("")).with("content-type", "text/html");
    }
    Resp::new(404, "")
}

/// Dev: the gate asks for the password.
fn dev(req: &Req) -> Resp {
    serve(req, true)
}

/// Prod: no gate.
fn prod(req: &Req) -> Resp {
    serve(req, false)
}

// ------------------------------------------------------------------ the binary

/// The real binary, killed when dropped.
struct Service {
    child: Child,
    port: u16,
}

impl Service {
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A site root holding a fake wasm under both names the page may link: a
/// debug build links `/pkg/vizualisation_bg.wasm` (search-front's test site
/// root holds `search-front_bg.wasm` for the same reason), a release one
/// `/pkg/vizualisation.wasm`; and, with it, the stylesheet the shell links
/// when files are not hashed, `/pkg/vizualisation.css`.
fn site_root(name: &str, with_wasm: bool) -> PathBuf {
    let dir = temp(&format!("site-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pkg")).expect("a site root");
    if with_wasm {
        for file in ["pkg/vizualisation_bg.wasm", "pkg/vizualisation.wasm"] {
            std::fs::write(dir.join(file), b"\0asm\x01\0\0\0").expect("wasm");
        }
        std::fs::write(dir.join("pkg/vizualisation.css"), b"body{}").expect("css");
    }
    dir
}

/// How the real service is configured: the gate, and the origin check.
struct Setup {
    gate: &'static str,
    secret: bool,
}

/// Starts the real service on `site` as `setup` says.
fn start_service(site: &Path, setup: &Setup) -> Service {
    let port = free_port();
    let mut command = Command::new(env!("CARGO_BIN_EXE_vizualisation"));
    command
        .env_remove("VIZ_GATE")
        .env_remove("DEV_PASSWORD")
        .env_remove("ORIGIN_SECRET")
        .env_remove("LEPTOS_ENV")
        .env_remove("K_SERVICE")
        .env("VIZ_ENV", RELEASE_ENV)
        .env("VIZ_SHA", RELEASE_SHA)
        .env("VIZ_GATE", setup.gate)
        .env("LEPTOS_OUTPUT_NAME", "vizualisation")
        .env("LEPTOS_SITE_ROOT", site)
        .env("VIZ_DATA_DIR", DATA_DIR)
        .env("LEPTOS_SITE_ADDR", format!("127.0.0.1:{port}"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if setup.gate == "password" {
        command.env("DEV_PASSWORD", PASSWORD);
    }
    if setup.secret {
        command.env("ORIGIN_SECRET", SECRET);
    }
    let child = command.spawn().expect("the binary starts");
    let mut service = Service { child, port };
    let deadline = Instant::now() + Duration::from_secs(30);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(
            service.child.try_wait().expect("status").is_none(),
            "stopped"
        );
        assert!(Instant::now() < deadline, "never listened");
        std::thread::sleep(Duration::from_millis(50));
    }
    service
}

// ------------------------------------------------------------------- helpers

/// Runs bash on `script` with `args` and the environment of a CI step.
fn run(script: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new("bash");
    command
        .arg(script)
        .args(args)
        .env_remove("ORIGIN_SECRET")
        .env_remove("DEV_PASSWORD")
        .env_remove("VIZ_GATE")
        .env_remove("LEPTOS_ENV")
        .env_remove("K_SERVICE")
        .env_remove("ROLLOUT_EXPECT_ENV")
        .env_remove("ROLLOUT_EXPECT_SHA")
        .env("SMOKE_WAIT", "8");
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("bash runs the script")
}

/// `env` plus the release a direct-mode `rollout-check.sh` is told to
/// expect (`ROLLOUT_EXPECT_ENV`, `ROLLOUT_EXPECT_SHA`): the one the fakes
/// and the real binary serve.
fn expect_release<'a>(env: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut all = env.to_vec();
    all.push(("ROLLOUT_EXPECT_ENV", RELEASE_ENV));
    all.push(("ROLLOUT_EXPECT_SHA", RELEASE_SHA));
    all
}

/// Everything the script printed.
fn printed(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The script failed (exit 1), named `check`, and printed no secret and
/// no password.
fn assert_fails_naming(output: &Output, check: &str, case: &str) {
    let text = printed(output);
    assert_eq!(output.status.code(), Some(1), "{case}: {text}");
    assert!(
        text.contains(&format!("FAIL {check}")),
        "{case}: names {check}: {text}"
    );
    assert!(!text.contains(SECRET), "{case}: secret printed: {text}");
    assert!(!text.contains(PASSWORD), "{case}: password printed: {text}");
}

/// A fake server that breaks one check, with the check it breaks and what
/// is wrong.
type Handler = Box<dyn Fn(&Req) -> Resp + Send>;
type Case = (&'static str, &'static str, Handler);

/// A dev server that answers `path`, to a caller with the secret and the
/// password, with `status` and `body`, and is healthy for the rest.
fn answering(path: &'static str, status: u16, body: &str) -> Handler {
    let body = body.to_owned();
    Box::new(move |r| {
        if r.authed() && r.admitted() && r.path == path {
            Resp::new(status, &body)
        } else {
            dev(r)
        }
    })
}

/// A dev server whose `/health`, to a caller with the secret, answers
/// `status` and `body`.
fn health(status: u16, body: &str) -> Handler {
    let body = body.to_owned();
    Box::new(move |r| {
        if r.authed() && r.path == "/health" {
            Resp::new(status, &body)
        } else {
            dev(r)
        }
    })
}

/// The ways a dev deployment breaks the checks both scripts share.
fn shared_cases() -> Vec<Case> {
    let mut cases = origin_cases();
    cases.extend(gate_and_page_cases());
    cases
}

/// `/health` and the origin guard.
fn origin_cases() -> Vec<Case> {
    vec![
        ("health", "/health is a 500", health(500, "boom")),
        ("health", "/health does not say ok", health(200, "starting")),
        (
            "guard",
            "/ without the header answers 200",
            Box::new(|r| {
                if r.path == "/" && r.admitted() {
                    Resp::new(200, &page("")).with("content-type", "text/html")
                } else {
                    dev(r)
                }
            }),
        ),
        (
            "guard",
            "a wrong header answers 200",
            Box::new(|r| {
                if r.header("x-origin-auth").is_some_and(|v| v != SECRET) && r.admitted() {
                    Resp::new(200, &page(""))
                } else {
                    dev(r)
                }
            }),
        ),
        (
            "guard",
            "/health without the header answers 200",
            Box::new(|r| {
                if r.path == "/health" {
                    Resp::new(200, HEALTHZ)
                } else {
                    dev(r)
                }
            }),
        ),
    ]
}

/// The password gate, the page, the wasm and the stylesheet.
fn gate_and_page_cases() -> Vec<Case> {
    vec![
        (
            "gate",
            "/ without the password answers 200",
            Box::new(|r| {
                if r.authed() && r.path == "/" {
                    Resp::new(200, &page("")).with("content-type", "text/html")
                } else {
                    dev(r)
                }
            }),
        ),
        (
            "gate",
            "the 401 asks for no password",
            Box::new(|r| {
                if r.authed() && r.path != "/health" && !r.admitted() {
                    Resp::new(401, "")
                } else {
                    dev(r)
                }
            }),
        ),
        ("root", "/ is a 500", answering("/", 500, "boom")),
        (
            "root",
            "/ does not name the page",
            answering("/", 200, "<html><title>nginx</title></html>"),
        ),
        ("wasm", "the wasm is a 404", answering(WASM, 404, "")),
        // The page links a stylesheet the site does not hold (a release
        // build's hashed file, linked by its unhashed name): no style.
        ("css", "the stylesheet is a 404", answering(CSS, 404, "")),
        (
            "css",
            "the stylesheet is served as text",
            answering(CSS, 200, "body{}"),
        ),
        (
            "css",
            "the page links no stylesheet",
            answering(
                "/",
                200,
                &format!(
                    "<html><head><title>Suttas Map</title>{}\
                     <link rel=\"preload\" href=\"{WASM}\"></head></html>",
                    release_metas(RELEASE_ENV, RELEASE_SHA)
                ),
            ),
        ),
    ]
}

/// The committed data files the binary reads at boot.
const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../public");

// ------------------------------------------------------------------- smoke.sh

/// The smoke test's environment for a binary: the site root it serves.
fn smoke_env(site: &Path) -> Vec<(&'static str, String)> {
    vec![
        ("LEPTOS_OUTPUT_NAME", "vizualisation".to_owned()),
        ("LEPTOS_SITE_ROOT", site.to_str().expect("utf-8").to_owned()),
        ("VIZ_DATA_DIR", DATA_DIR.to_owned()),
    ]
}

/// `smoke.sh BINARY` with the site root.
fn smoke_binary(binary: &str, site: &Path) -> Output {
    let env = smoke_env(site);
    let env: Vec<(&str, &str)> = env.iter().map(|(n, v)| (*n, v.as_str())).collect();
    run(&script("smoke.sh"), &[binary], &env)
}

#[test]
fn smoke_passes_on_the_real_binary_and_leaves_no_service_behind() {
    let site = site_root("smoke-ok", true);
    let out = smoke_binary(env!("CARGO_BIN_EXE_vizualisation"), &site);
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    for check in [
        "health ok",
        "guard ok",
        "gate ok",
        "root ok",
        "wasm ok",
        "css ok",
        "prod ok",
        "boot ok",
        "leak ok",
    ] {
        assert!(text.contains(check), "{check} is reported: {text}");
    }
    let _ = std::fs::remove_dir_all(site);
}

#[test]
fn smoke_fails_naming_the_wasm_when_it_is_not_served() {
    let site = site_root("smoke-nowasm", false);
    let out = smoke_binary(env!("CARGO_BIN_EXE_vizualisation"), &site);
    assert_fails_naming(&out, "wasm", "no wasm in the site root");
    let _ = std::fs::remove_dir_all(site);
}

/// The page links a stylesheet the site root does not hold (what a release
/// build with hashed files did while the app linked the unhashed name).
#[test]
fn smoke_fails_naming_the_css_when_it_is_not_served() {
    let site = site_root("smoke-nocss", true);
    std::fs::remove_file(site.join("pkg/vizualisation.css")).expect("the css is removed");
    let out = smoke_binary(env!("CARGO_BIN_EXE_vizualisation"), &site);
    assert_fails_naming(&out, "css", "no stylesheet in the site root");
    let _ = std::fs::remove_dir_all(site);
}

#[test]
fn smoke_fails_when_the_binary_dies_at_boot() {
    let out = run(&script("smoke.sh"), &["/usr/bin/false"], &[]);
    assert_fails_naming(&out, "listen", "a binary that exits");
}

/// A wrapper around the real binary that plays a boot gone wrong.
fn wrapper(name: &str, body: &str) -> PathBuf {
    let path = temp(&format!("wrapper-{name}"));
    let real = env!("CARGO_BIN_EXE_vizualisation");
    std::fs::write(
        &path,
        format!("#!/usr/bin/env bash\n{body}\nexec '{real}'\n"),
    )
    .expect("a wrapper");
    Command::new("chmod")
        .args(["+x", path.to_str().expect("utf-8")])
        .status()
        .expect("chmod");
    path
}

/// The boot must refuse an unset `VIZ_GATE` with exit 2 and never listen: a
/// binary that defaults the gate, or refuses with another code, fails `boot`.
#[test]
fn smoke_fails_naming_the_boot_when_an_unset_gate_is_not_refused() {
    let site = site_root("smoke-boot", true);
    for (name, body) in [
        ("defaults-off", "export VIZ_GATE=${VIZ_GATE:-off}"),
        ("exits-1", "[ -n \"${VIZ_GATE:-}\" ] || exit 1"),
    ] {
        let binary = wrapper(name, body);
        let out = smoke_binary(binary.to_str().expect("utf-8"), &site);
        assert_fails_naming(&out, "boot", name);
        let _ = std::fs::remove_file(binary);
    }
    let _ = std::fs::remove_dir_all(site);
}

/// The prod run (`check_prod`) starts the binary as prod does: the gate
/// off, `LEPTOS_ENV=PROD`, the origin secret, no password. A wrapper that
/// refuses any other environment still passes, and reports `prod ok`.
#[test]
fn smoke_runs_the_binary_as_prod_does() {
    let site = site_root("smoke-prod-env", true);
    let binary = wrapper(
        "prod-env",
        "if [ \"${VIZ_GATE:-}\" = off ]; then\n\
         [ \"${LEPTOS_ENV:-}\" = PROD ] && [ -n \"${ORIGIN_SECRET:-}\" ] \\\n\
         && [ -z \"${DEV_PASSWORD+set}\" ] && [ -n \"${VIZ_DATA_DIR:-}\" ] || exit 3\n\
         fi",
    );
    let out = smoke_binary(binary.to_str().expect("utf-8"), &site);
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("prod ok"), "{text}");
    let _ = std::fs::remove_file(binary);
    let _ = std::fs::remove_dir_all(site);
}

/// A binary that asks for a password with the gate off (prod), or does not
/// stop on a SIGTERM there, fails `prod`; one that writes the secret or the password to its output fails `leak`,
/// naming the log (the dev run's `server.log`, the refused boot's
/// `boot.log`).
#[test]
fn smoke_fails_on_a_password_in_prod_and_on_a_secret_in_a_log() {
    let site = site_root("smoke-prod-leak", true);
    for (name, body, check, file) in [
        (
            "prod-401",
            "[ \"${VIZ_GATE:-}\" != off ] || export VIZ_GATE=password DEV_PASSWORD=wrapper-pw",
            "prod",
            "prod-",
        ),
        (
            "prod-ignores-sigterm",
            "[ \"${VIZ_GATE:-}\" != off ] || trap '' TERM",
            "prod",
            "SIGTERM",
        ),
        (
            "server-log",
            "[ \"${VIZ_GATE:-}\" != password ] || echo \"debug: origin $ORIGIN_SECRET\" >&2",
            "leak",
            "server.log",
        ),
        (
            "boot-log",
            "[ -n \"${VIZ_GATE:-}\" ] || echo \"debug: password $DEV_PASSWORD\" >&2",
            "leak",
            "boot.log",
        ),
    ] {
        let binary = wrapper(name, body);
        let out = smoke_binary(binary.to_str().expect("utf-8"), &site);
        assert_fails_naming(&out, check, name);
        assert!(
            printed(&out).contains(file),
            "{name}: names {file}: {}",
            printed(&out)
        );
        let _ = std::fs::remove_file(binary);
    }
    let _ = std::fs::remove_dir_all(site);
}

/// What only `smoke.sh` checks: the probe's body, the wasm's type, and no
/// echo of a secret.
fn smoke_only_cases() -> Vec<Case> {
    vec![
        (
            "health",
            "/health gives no stamp line",
            health(200, "ok 63bdba3\n"),
        ),
        (
            "health",
            "/health says ok without a commit",
            health(200, "ok\ngraph.json: none\n"),
        ),
        (
            "wasm",
            "the wasm is served as text",
            answering(WASM, 200, "<html>"),
        ),
        (
            "gate",
            "the wasm is served without the password",
            Box::new(|r| {
                if r.authed() && r.path == WASM {
                    Resp::new(200, "\0asm").with("content-type", "application/wasm")
                } else {
                    dev(r)
                }
            }),
        ),
        (
            "leak",
            "the page echoes the origin secret",
            answering("/", 200, &page(SECRET)),
        ),
        (
            "leak",
            "a 401 echoes the password in a header",
            Box::new(|r| {
                let resp = dev(r);
                if resp.status == 401 {
                    resp.with("x-debug", PASSWORD)
                } else {
                    resp
                }
            }),
        ),
    ]
}

/// `smoke.sh --port` against a fake server: the passing one, then one that
/// breaks each check in turn.
#[test]
fn smoke_against_a_server_names_the_check_that_failed() {
    let env = [("ORIGIN_SECRET", SECRET), ("DEV_PASSWORD", PASSWORD)];
    let good = Fake::start(dev);
    let out = run(
        &script("smoke.sh"),
        &["--port", &good.port.to_string()],
        &env,
    );
    assert_eq!(out.status.code(), Some(0), "healthy: {}", printed(&out));

    let mut cases = shared_cases();
    cases.extend(smoke_only_cases());
    for (check, case, handler) in cases {
        let fake = Fake::start(handler);
        let out = run(
            &script("smoke.sh"),
            &["--port", &fake.port.to_string()],
            &env,
        );
        assert_fails_naming(&out, check, case);
    }
}

#[test]
fn smoke_against_a_server_needs_both_secrets_in_the_environment() {
    let fake = Fake::start(dev);
    let port = fake.port.to_string();
    for (missing, env) in [
        ("ORIGIN_SECRET", [("DEV_PASSWORD", PASSWORD)]),
        ("DEV_PASSWORD", [("ORIGIN_SECRET", SECRET)]),
    ] {
        let out = run(&script("smoke.sh"), &["--port", &port], &env);
        assert_eq!(out.status.code(), Some(2), "{}", printed(&out));
        assert!(printed(&out).contains(missing), "{}", printed(&out));
    }
    assert_eq!(fake.seen.lock().expect("lock").len(), 0, "nothing was sent");
}

// ---------------------------------------------------------- rollout-check.sh

fn check_script() -> PathBuf {
    script("rollout-check.sh")
}

/// A header file as the CI step writes it: `x-origin-auth: <secret>`.
fn header_file(name: &str) -> PathBuf {
    let path = temp(&format!("origin-header-{name}"));
    std::fs::write(&path, format!("x-origin-auth: {SECRET}\n")).expect("a header file");
    path
}

/// The password file the dev deploy writes: `authorization: Basic <b64>`.
fn password_file(name: &str) -> PathBuf {
    let path = temp(&format!("password-header-{name}"));
    let basic = STANDARD.encode(format!("deploy:{PASSWORD}"));
    std::fs::write(&path, format!("authorization: Basic {basic}\n")).expect("a password file");
    path
}

fn utf8(path: &Path) -> &str {
    path.to_str().expect("utf-8")
}

/// Direct mode reports every check by name.
fn assert_every_check_reported(text: &str) {
    for check in [
        "health ok",
        "gate ok",
        "root ok",
        "release ok",
        "wasm ok",
        "css ok",
        "guard ok",
    ] {
        assert!(text.contains(check), "{check} is reported: {text}");
    }
}

#[test]
fn rollout_check_passes_on_the_real_dev_binary() {
    let site = site_root("rollout-dev", true);
    let pass = password_file("real-dev");
    let file = header_file("real-dev");

    // Dev: the gate on, the origin checked.
    let dev_service = start_service(
        &site,
        &Setup {
            gate: "password",
            secret: true,
        },
    );
    let url = dev_service.url();
    let out = run(
        &check_script(),
        &["--password-file", utf8(&pass), &url],
        &expect_release(&[("ORIGIN_SECRET", SECRET)]),
    );
    let text = printed(&out);
    assert_eq!(
        out.status.code(),
        Some(0),
        "dev, secret from the env: {text}"
    );
    assert_every_check_reported(&text);
    let out = run(
        &check_script(),
        &["--password-file", utf8(&pass), &url, utf8(&file)],
        &expect_release(&[]),
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "dev, secret from a file: {}",
        printed(&out)
    );
    let out = run(&check_script(), &["--refused", &url], &[]);
    assert_eq!(out.status.code(), Some(0), "refused: {}", printed(&out));
    // Without the password file the check is prod's: dev's 401 fails it.
    let out = run(&check_script(), &[&url, utf8(&file)], &expect_release(&[]));
    assert_fails_naming(&out, "root", "dev checked as prod");
    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_file(pass);
    let _ = std::fs::remove_dir_all(site);
}

#[test]
fn rollout_check_passes_on_the_real_prod_binary_and_through_the_worker() {
    let site = site_root("rollout-prod", true);
    let pass = password_file("real-prod");
    let file = header_file("real-prod");

    // Prod: no gate, the origin checked.
    let prod_service = start_service(
        &site,
        &Setup {
            gate: "off",
            secret: true,
        },
    );
    let out = run(
        &check_script(),
        &[&prod_service.url(), utf8(&file)],
        &expect_release(&[]),
    );
    assert_eq!(out.status.code(), Some(0), "prod: {}", printed(&out));

    // Through the Worker the service is reached without the header: here a
    // service that checks no origin stands for it.
    for (gate, args) in [
        (
            "password",
            vec!["--via-worker", "--password-file", utf8(&pass)],
        ),
        ("off", vec!["--via-worker"]),
    ] {
        let open = start_service(
            &site,
            &Setup {
                gate,
                secret: false,
            },
        );
        let open_url = open.url();
        let mut all = args.clone();
        all.push(&open_url);
        let chrome = FakeChrome::new(&format!("real-{gate}"));
        let out = run(&check_script(), &all, &chrome.env("ok"));
        assert_eq!(
            out.status.code(),
            Some(0),
            "via the Worker, gate {gate}: {}",
            printed(&out)
        );
        assert!(
            printed(&out).contains("hydrate ok"),
            "the Worker mode checks hydration, gate {gate}: {}",
            printed(&out)
        );
    }
    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_file(pass);
    let _ = std::fs::remove_dir_all(site);
}

/// Each way a dev rollout can be wrong has a server that must fail the check
/// that guards it, and the failure names that check.
#[test]
fn rollout_check_names_the_check_that_failed() {
    let pass = password_file("cases");
    for (check, case, handler) in shared_cases() {
        let fake = Fake::start(handler);
        let out = run(
            &check_script(),
            &["--password-file", utf8(&pass), &fake.url()],
            &expect_release(&[("ORIGIN_SECRET", SECRET)]),
        );
        assert_fails_naming(&out, check, case);
    }
    let _ = std::fs::remove_file(pass);
}

/// Prod answers with no `WWW-Authenticate`: a gate left on, or a header that
/// asks for a password, fails `gate`.
#[test]
fn rollout_check_on_prod_refuses_a_password_prompt() {
    let good = Fake::start(prod);
    let out = run(
        &check_script(),
        &[&good.url()],
        &expect_release(&[("ORIGIN_SECRET", SECRET)]),
    );
    assert_eq!(out.status.code(), Some(0), "prod: {}", printed(&out));
    let asking =
        Fake::start(|r| prod(r).with("www-authenticate", "Basic realm=\"PaliCanon viz dev\""));
    let out = run(
        &check_script(),
        &[&asking.url()],
        &expect_release(&[("ORIGIN_SECRET", SECRET)]),
    );
    assert_fails_naming(&out, "gate", "prod asks for a password");
    let gated = Fake::start(dev);
    let out = run(
        &check_script(),
        &[&gated.url()],
        &expect_release(&[("ORIGIN_SECRET", SECRET)]),
    );
    assert_fails_naming(&out, "root", "prod behind the gate");
}

#[test]
fn rollout_check_demands_a_secret_and_refuses_to_send_it_to_the_worker() {
    let fake = Fake::start(dev);
    let out = run(&check_script(), &[&fake.url()], &expect_release(&[]));
    assert_eq!(out.status.code(), Some(2), "no secret: {}", printed(&out));
    let file = header_file("worker");
    let out = run(
        &check_script(),
        &["--via-worker", &fake.url(), utf8(&file)],
        &[],
    );
    assert_eq!(out.status.code(), Some(2), "{}", printed(&out));
    let out = run(
        &check_script(),
        &["--via-worker", &fake.url()],
        &[("ORIGIN_SECRET", SECRET)],
    );
    assert_eq!(out.status.code(), Some(2), "{}", printed(&out));
    let bad = temp("rollout-bad");
    std::fs::write(&bad, "authorization: Bearer x\n").expect("a file");
    let out = run(
        &check_script(),
        &[&fake.url(), utf8(&bad)],
        &expect_release(&[]),
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "a file of another header: {}",
        printed(&out)
    );
    assert_eq!(fake.seen.lock().expect("lock").len(), 0, "nothing was sent");
    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_file(bad);
}

/// The password file is one `authorization: Basic …` line, a file, and is
/// never sent to the bare origin (`--refused` sends nothing of ours).
#[test]
fn rollout_check_takes_the_password_as_a_file_of_one_basic_line() {
    let fake = Fake::start(dev);
    let pass = password_file("refused");
    let out = run(
        &check_script(),
        &["--refused", "--password-file", utf8(&pass), &fake.url()],
        &[],
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "refused with a password: {}",
        printed(&out)
    );
    let origin = header_file("as-password");
    let missing = temp("no-such-password-file");
    for (case, file) in [
        ("another header", utf8(&origin)),
        ("no file", utf8(&missing)),
    ] {
        let out = run(
            &check_script(),
            &["--password-file", file, &fake.url()],
            &expect_release(&[("ORIGIN_SECRET", SECRET)]),
        );
        assert_eq!(out.status.code(), Some(2), "{case}: {}", printed(&out));
    }
    let out = run(&check_script(), &["--password-file"], &[]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "no file named: {}",
        printed(&out)
    );
    assert_eq!(fake.seen.lock().expect("lock").len(), 0, "nothing was sent");
    let _ = std::fs::remove_file(pass);
    let _ = std::fs::remove_file(origin);
}

#[test]
fn rollout_check_refused_mode_fails_on_a_server_that_answers() {
    let open = Fake::start(|r| prod(&r.through_worker()));
    let out = run(&check_script(), &["--refused", &open.url()], &[]);
    assert_fails_naming(&out, "refused", "an open origin");
    let wordy = Fake::start(|_| Resp::new(403, "forbidden: send x-origin-auth"));
    let out = run(&check_script(), &["--refused", &wordy.url()], &[]);
    assert_fails_naming(&out, "refused", "a 403 with a body");
}

/// R6 (search-front): Cloudflare's challenge is told apart from our own
/// refusal.
#[test]
fn rollout_check_tells_a_cloudflare_challenge_from_a_failure() {
    let challenge = Fake::start(|_| Resp::new(403, "challenge").with("cf-mitigated", "challenge"));
    let out = run(&check_script(), &["--via-worker", &challenge.url()], &[]);
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(3), "{text}");
    assert!(text.contains("challenge"), "{text}");
}

/// Exit 4: the Worker itself answered 500 (it has no secret or no origin;
/// `origin-secret push` is the remedy), told from a 500 of the service that
/// the Worker relays (a bad rollout: exit 1) and from a refusal (403). The
/// first request is `/health`, so that is the check a relayed failure names.
#[test]
fn rollout_check_tells_the_workers_500_from_a_failure() {
    for body in [
        "The origin secret is not configured.",
        "The origin is not configured.",
    ] {
        let worker = Fake::start(move |_| Resp::new(500, body).with("cache-control", "no-store"));
        let out = run(&check_script(), &["--via-worker", &worker.url()], &[]);
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(4), "{body}: {text}");
        assert!(text.contains("Worker"), "{body}: names the Worker: {text}");
    }
    let relayed = Fake::start(|_| Resp::new(500, "boom"));
    let out = run(&check_script(), &["--via-worker", &relayed.url()], &[]);
    assert_fails_naming(&out, "health", "the service's own 500, relayed");
    let refusal = Fake::start(|_| Resp::new(403, "forbidden"));
    let out = run(&check_script(), &["--via-worker", &refusal.url()], &[]);
    assert_fails_naming(&out, "health", "a 403");
}

/// The Worker's 500 is recognised on the suffix every one of its bodies
/// shares, `is not configured.`, after trimming whitespace and CR: a proxy
/// or a hop may add a CRLF, a trailing newline or padding.
#[test]
fn rollout_check_recognises_the_workers_500_through_crlf_and_newlines() {
    for body in [
        "The origin secret is not configured.\r\n",
        "The origin secret is not configured.\n",
        "The origin secret is not configured.\n\n",
        "The origin is not configured.\r",
        "The public origin is not configured.",
        "The public origin is not configured.\r\n",
        "  The public origin is not configured.  \r\n",
        "\r\nThe origin secret is not configured.\r\n\r\n",
    ] {
        let worker = Fake::start(move |_| Resp::new(500, body).with("cache-control", "no-store"));
        let out = run(&check_script(), &["--via-worker", &worker.url()], &[]);
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(4), "{body:?}: {text}");
        assert!(
            text.contains("Worker"),
            "{body:?}: names the Worker: {text}"
        );
    }
}

/// A 500 of the service, relayed, is a failed check even when it is close:
/// the suffix must end the body, and must be a whole sentence's end.
#[test]
fn rollout_check_does_not_take_a_service_500_for_the_workers() {
    for body in [
        "boom",
        "",
        "is not configured. and then some",
        "The origin is not configured, sorry",
        "The origin is not configured",
    ] {
        let relayed = Fake::start(move |_| Resp::new(500, body));
        let out = run(&check_script(), &["--via-worker", &relayed.url()], &[]);
        assert_fails_naming(&out, "health", &format!("the service's own 500 {body:?}"));
    }
}

/// Only through the Worker is a 500 the Worker's: a direct call to a
/// revision that answers the same words is a failing check.
#[test]
fn rollout_check_direct_mode_has_no_worker_500() {
    let worker_like = Fake::start(|_| Resp::new(500, "The origin secret is not configured."));
    let out = run(
        &check_script(),
        &[&worker_like.url()],
        &expect_release(&[("ORIGIN_SECRET", SECRET)]),
    );
    assert_fails_naming(&out, "health", "direct mode");
}

// ------------------------------------------------ rollout-check.sh: release

/// A prod fake whose `/` is `html` for a caller with the secret; the rest
/// is `prod`.
fn serving(html: String) -> Fake {
    Fake::start(move |r| {
        if r.authed() && r.path == "/" {
            Resp::new(200, &html).with("content-type", "text/html")
        } else {
            prod(r)
        }
    })
}

/// The gate against `fake`, with the origin secret in the environment and
/// the intended release as given (`None`: the variable is not set).
fn release_run(fake: &Fake, env: Option<&str>, sha: Option<&str>) -> Output {
    let mut vars = vec![("ORIGIN_SECRET", SECRET)];
    if let Some(value) = env {
        vars.push(("ROLLOUT_EXPECT_ENV", value));
    }
    if let Some(value) = sha {
        vars.push(("ROLLOUT_EXPECT_SHA", value));
    }
    run(&check_script(), &[&fake.url()], &vars)
}

/// What the gate accepts: the metas the page carries, and the release it
/// is told to expect (env, sha).
type Passing = (&'static str, String, &'static str, &'static str);

/// Pages that serve the intended release, however the metas are written
/// (task 016, from learn-pali 060).
fn passing_releases() -> Vec<Passing> {
    let mut all = passing_values();
    all.extend(passing_spellings());
    all
}

/// The intended release as an env and a sha: main, a tag, a branch name.
fn passing_values() -> Vec<Passing> {
    vec![
        (
            "the metas match",
            release_metas("main", "34c489c"),
            "main",
            "34c489c",
        ),
        (
            "a tag: an empty sha",
            release_metas("v0.1.8", ""),
            "v0.1.8",
            "",
        ),
        (
            "a branch name with a slash",
            release_metas("feat/x", "34c489c"),
            "feat/x",
            "34c489c",
        ),
    ]
}

/// The same release, with the metas written in other ways.
fn passing_spellings() -> Vec<Passing> {
    let decoys = "<meta name=\"description\" content=\"viz-release-env main\">\
                  <meta name=\"viz-release-env-x\" content=\"zzz\">\
                  <meta property=\"viz-release-env\" content=\"zzz\">\
                  <meta data-name=\"viz-release-env\" content=\"zzz\">";
    vec![
        (
            "content before name, a self-closing tag",
            "<meta content=\"main\" name=\"viz-release-env\">\
             <meta content=\"34c489c\" name=\"viz-release-sha\"/>"
                .to_owned(),
            "main",
            "34c489c",
        ),
        (
            "single-quoted attributes",
            "<meta name='viz-release-env' content='main'>\
             <meta name='viz-release-sha' content='34c489c'>"
                .to_owned(),
            "main",
            "34c489c",
        ),
        (
            "a tag split over lines",
            "<meta\n  name=\"viz-release-env\"\n  content=\"main\"\n>\
             <meta\n  name=\"viz-release-sha\"\n  content=\"34c489c\"\n>"
                .to_owned(),
            "main",
            "34c489c",
        ),
        (
            "lookalike metas are not the metas",
            format!("{decoys}{}", release_metas("main", "34c489c")),
            "main",
            "34c489c",
        ),
    ]
}

/// The gate passes a page that serves the intended release, however its
/// metas are written, and says `release ok`; a tag expects an empty sha.
#[test]
fn rollout_check_release_passes_when_the_page_serves_the_intended_release() {
    for (case, metas, env, sha) in passing_releases() {
        let fake = serving(page_with(&metas, ""));
        let out = release_run(&fake, Some(env), Some(sha));
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(0), "{case}: {text}");
        assert!(text.contains("rollout-check: release ok"), "{case}: {text}");
    }
}

/// A page that serves another release: its metas, the release the gate is
/// told to expect (env, sha) and the fragments the message must hold.
type Failing = (
    &'static str,
    String,
    &'static str,
    &'static str,
    Vec<&'static str>,
);

/// The shared message of a meta that has no `content`.
const NO_CONTENT: &str = "has no content attribute";

/// Pages that serve another release, or none, with what the exit-8 message
/// must say.
fn failing_releases() -> Vec<Failing> {
    let mut all = failing_values();
    all.extend(failing_metas());
    all
}

/// The metas are there and say another release.
fn failing_values() -> Vec<Failing> {
    let ok = |env: &str| release_metas(env, "34c489c");
    vec![
        (
            "env wrong",
            ok("dev"),
            "main",
            "34c489c",
            vec!["viz-release-env", "'main'", "'dev'"],
        ),
        (
            "sha wrong",
            release_metas("main", "deadbee"),
            "main",
            "34c489c",
            vec!["viz-release-sha", "'34c489c'", "'deadbee'"],
        ),
        (
            "a tag expected, the page serves main's sha",
            release_metas("v0.1.8", "34c489c"),
            "v0.1.8",
            "",
            vec!["viz-release-sha"],
        ),
        (
            "a sha expected, the page serves an empty one",
            release_metas("main", ""),
            "main",
            "34c489c",
            vec!["viz-release-sha"],
        ),
        (
            "a prefix of the env is not the env",
            ok("feat"),
            "feat/x",
            "34c489c",
            vec!["viz-release-env"],
        ),
    ]
}

/// A meta is missing, or has no `content`.
fn failing_metas() -> Vec<Failing> {
    vec![
        (
            "the env meta is missing",
            "<meta name=\"viz-release-sha\" content=\"34c489c\">".to_owned(),
            "main",
            "34c489c",
            vec!["no <meta name=\"viz-release-env\">", "'main'"],
        ),
        (
            "the sha meta is missing while an empty sha is expected",
            "<meta name=\"viz-release-env\" content=\"v0.1.8\">".to_owned(),
            "v0.1.8",
            "",
            vec!["no <meta name=\"viz-release-sha\">"],
        ),
        (
            "the env meta has no content",
            "<meta name=\"viz-release-env\"><meta name=\"viz-release-sha\" content=\"34c489c\">"
                .to_owned(),
            "main",
            "34c489c",
            vec!["viz-release-env", NO_CONTENT],
        ),
        (
            "the sha meta has no content",
            "<meta name=\"viz-release-env\" content=\"main\"><meta name=\"viz-release-sha\">"
                .to_owned(),
            "main",
            "34c489c",
            vec!["viz-release-sha", NO_CONTENT],
        ),
        (
            "a tag: the sha meta has no content attribute at all (not an empty sha)",
            "<meta name=\"viz-release-env\" content=\"v0.1.8\"><meta name=\"viz-release-sha\">"
                .to_owned(),
            "v0.1.8",
            "",
            vec!["viz-release-sha", NO_CONTENT],
        ),
    ]
}

/// A wrong release is exit 8 (`release`), never a flaky curl (1): the env
/// or the sha differs, a meta is absent or has no `content`. It guards the
/// two metas and the exit code; it does not guard the message's wording
/// beyond the fragments listed.
#[test]
fn rollout_check_release_exits_8_when_the_page_serves_another_release() {
    for (case, metas, env, sha, fragments) in failing_releases() {
        let fake = serving(page_with(&metas, ""));
        let out = release_run(&fake, Some(env), Some(sha));
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(8), "{case}: {text}");
        assert!(
            text.contains("FAIL release"),
            "{case}: names release: {text}"
        );
        for fragment in fragments {
            assert!(text.contains(fragment), "{case}: {fragment}: {text}");
        }
        assert!(!text.contains(SECRET), "{case}: secret printed: {text}");
    }
}

/// The release is judged after the page is named and before the wasm: a
/// wrong release never reaches the wasm, and a page that is not the map is
/// the root's failure (1), whatever its metas.
#[test]
fn rollout_check_release_is_judged_after_the_root_and_before_the_wasm() {
    let wrong = serving(page_with(&release_metas("dev", "34c489c"), ""));
    let out = release_run(&wrong, Some("main"), Some("34c489c"));
    assert_eq!(out.status.code(), Some(8), "{}", printed(&out));
    let seen = wrong.seen.lock().expect("lock");
    assert!(seen.iter().any(|r| r.path == "/"), "the page was read");
    assert!(
        seen.iter().all(|r| r.path != WASM),
        "the wasm was not asked"
    );
    drop(seen);
    let unnamed = serving(format!(
        "<html><head><title>nginx</title>{}</head></html>",
        release_metas("dev", "34c489c")
    ));
    let out = release_run(&unnamed, Some("main"), Some("34c489c"));
    assert_fails_naming(&out, "root", "a wrong env on a page that is not the map");
}

/// The served value is shown printable, cut to 100 characters, and never
/// as the origin secret: a page that echoes the secret as its env, from the
/// environment or from a header file, prints `<the origin secret>`.
#[test]
fn rollout_check_never_prints_the_origin_secret_in_a_release_message() {
    let echo = serving(page_with(&release_metas(SECRET, "34c489c"), ""));
    let out = release_run(&echo, Some("main"), Some("34c489c"));
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(8), "secret from the env: {text}");
    assert!(text.contains("<the origin secret>"), "{text}");
    assert!(!text.contains(SECRET), "secret printed: {text}");
    let file = header_file("release-echo");
    let out = run(
        &check_script(),
        &[&echo.url(), utf8(&file)],
        &[
            ("ROLLOUT_EXPECT_ENV", "main"),
            ("ROLLOUT_EXPECT_SHA", "34c489c"),
        ],
    );
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(8), "secret from a file: {text}");
    assert!(text.contains("<the origin secret>"), "{text}");
    assert!(!text.contains(SECRET), "secret printed: {text}");
    let _ = std::fs::remove_file(file);
}

/// What is shown of a served value: at most 100 printable characters.
#[test]
fn rollout_check_shows_a_served_release_short_and_printable() {
    let long = format!("{}TAIL", "a".repeat(100));
    let odd = "x\u{1b}[31my";
    for (case, value, kept, gone) in [
        (
            "cut to 100 characters",
            long.as_str(),
            "a".repeat(100),
            "TAIL",
        ),
        ("no control character", odd, "x[31my".to_owned(), "\u{1b}"),
    ] {
        let fake = serving(page_with(&release_metas(value, "34c489c"), ""));
        let out = release_run(&fake, Some("main"), Some("34c489c"));
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(8), "{case}: {text}");
        assert!(text.contains(&kept), "{case}: {kept}: {text}");
        assert!(!text.contains(gone), "{case}: {gone:?} printed: {text}");
    }
}

/// Direct mode needs the intended release before any request: the env
/// non-empty, the sha set (empty is a tag); a gate that silently skips is
/// no gate.
#[test]
fn rollout_check_release_needs_the_expected_release() {
    for (case, env, sha, name) in [
        (
            "ROLLOUT_EXPECT_ENV unset",
            None,
            Some("34c489c"),
            "ROLLOUT_EXPECT_ENV",
        ),
        (
            "ROLLOUT_EXPECT_ENV empty",
            Some(""),
            Some("34c489c"),
            "ROLLOUT_EXPECT_ENV",
        ),
        (
            "ROLLOUT_EXPECT_SHA unset (empty is fine, unset is not)",
            Some("main"),
            None,
            "ROLLOUT_EXPECT_SHA",
        ),
        ("both unset", None, None, "usage"),
    ] {
        let fake = Fake::start(prod);
        let out = release_run(&fake, env, sha);
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(2), "{case}: {text}");
        assert!(text.contains(name), "{case}: names {name}: {text}");
        assert_eq!(fake.seen.lock().expect("lock").len(), 0, "{case}: sent");
    }
}

/// `--refused` and `--via-worker` neither read nor need the intended
/// release, and do not fail when it is set (even to something the page
/// does not serve): they run no release check.
#[test]
fn rollout_check_via_worker_and_refused_need_no_expectation() {
    let closed = Fake::start(|_| Resp::new(403, ""));
    for vars in [
        vec![],
        vec![("ROLLOUT_EXPECT_ENV", "zzz"), ("ROLLOUT_EXPECT_SHA", "zzz")],
    ] {
        let out = run(&check_script(), &["--refused", &closed.url()], &vars);
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(0), "refused: {text}");
        assert!(text.contains("refused ok"), "{text}");
    }
    let open = Fake::start(prod_through_the_worker);
    let chrome = FakeChrome::new("no-expectation");
    for (case, extra) in [
        ("none", vec![]),
        (
            "a release the page does not serve",
            vec![("ROLLOUT_EXPECT_ENV", "zzz"), ("ROLLOUT_EXPECT_SHA", "zzz")],
        ),
    ] {
        let mut vars = chrome.env("ok");
        vars.extend(extra);
        let out = run(&check_script(), &["--via-worker", &open.url()], &vars);
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(0), "via the Worker, {case}: {text}");
        assert!(text.contains("hydrate ok"), "{case}: {text}");
        assert!(
            !text.contains("release"),
            "{case}: no release check: {text}"
        );
    }
}

// ------------------------------------------------- rollout-check.sh: hydrate

/// The DOM a hydrated page dumps: `<html data-hydrated>` (ui-lib's
/// `hydrate` sets it), Cloudflare's decoy first in `<body>`, the app's root
/// after it.
const HYDRATED_DOM: &str = "<html lang=\"en\" data-hydrated=\"\"><head></head><body>\
     <a href=\"/cdn-cgi/content?id=x\" style=\"display:none\"></a>\
     <div id=\"pc-app\"></div></body></html>";
/// The same page that never hydrated: no marker.
const BARE_DOM: &str =
    "<html lang=\"en\"><head></head><body><div id=\"pc-app\"></div></body></html>";
/// Cloudflare's managed challenge, as Chrome dumps it.
const CHALLENGE_DOM: &str = "<html><head><title>Just a moment...</title></head><body>\
     <script>window._cf_chl_opt={cType:'managed'};</script></body></html>";
/// What Chrome's `--enable-logging=stderr` prints for the panic of ui-lib
/// 063; the fake adds `, source: <a URL>` (in dev, the popup's URL, which
/// holds the password: the script must not print it).
const PANIC_LINE: &str =
    "[1:2:INFO:CONSOLE(1)] \"panicked at tachys-0.2.19/src/hydration.rs:227:9\"";
/// The pages the check loads.
const HYDRATE_PAGES: [&str; 3] = ["/", "/sim/mn10", "/privacy"];

/// A `chrome` that logs its arguments and plays `FAKE_CHROME_MODE` per page:
/// `ok`, `panic` (the panic on stderr, the marker still dumped: stderr is
/// read on its own), `runtime` (a wasm `RuntimeError`), `bare` (no marker),
/// `challenge`, `hang` (never answers), `privacy-panic` (`ok` but on
/// `/privacy`, which panics), `stuck` (dumps the launcher page itself, as
/// when its popup never opens). Given a `file://` page (dev: the launcher
/// that carries the password), it reads the URL the launcher's popup opens
/// (with the password) and the one it then goes to, logs them in
/// `targets.log` as `popup -> page`, and plays the page.
struct FakeChrome {
    dir: PathBuf,
    bin: PathBuf,
    log: PathBuf,
    targets: PathBuf,
}

impl FakeChrome {
    fn new(name: &str) -> FakeChrome {
        let dir = temp(&format!("fake-chrome-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a fake chrome dir");
        let log = dir.join("argv.log");
        let targets = dir.join("targets.log");
        let bin = dir.join("chrome");
        let body = format!(
            r#"#!/bin/bash
printf '%s\n' "$@" >> '{log}'
url="${{!#}}"
target="$url"
source="$url"
case "$url" in
  file://*)
    launcher="${{url#file://}}"
    source="$(sed -n 's/.*window\.open("\([^"]*\)").*/\1/p' "$launcher")"
    target="$(sed -n 's/.*location\.replace("\([^"]*\)").*/\1/p' "$launcher")"
    printf '%s -> %s\n' "$source" "$target" >> '{targets}'
    ;;
esac
mode="${{FAKE_CHROME_MODE:-ok}}"
if [ "$mode" = privacy-panic ]; then
  if [ "${{target%/privacy}}" != "$target" ]; then mode=panic; else mode=ok; fi
fi
case "$mode" in
  ok) printf '%s\n' '{HYDRATED_DOM}' ;;
  panic) printf '%s, source: %s\n' '{PANIC_LINE}' "$source" >&2; printf '%s\n' '{HYDRATED_DOM}' ;;
  runtime) printf '%s\n' 'Uncaught RuntimeError: unreachable' >&2; printf '%s\n' '{BARE_DOM}' ;;
  bare) printf '%s\n' '{BARE_DOM}' ;;
  challenge) printf '%s\n' '{CHALLENGE_DOM}' ;;
  hang) exec sleep 30 ;;
  stuck) cat "$launcher" ;;
esac
"#,
            log = log.display(),
            targets = targets.display(),
        );
        std::fs::write(&bin, body).expect("a fake chrome");
        Command::new("chmod")
            .args(["+x", utf8(&bin)])
            .status()
            .expect("chmod");
        FakeChrome {
            dir,
            bin,
            log,
            targets,
        }
    }

    /// The environment that makes `rollout-check.sh` run this Chrome in
    /// `mode`, with a limit of 3 s for `hang` and 30 s for the others (a
    /// loaded machine must not turn a pass into a timeout; 60 s in a
    /// deploy).
    fn env(&self, mode: &'static str) -> Vec<(&'static str, &str)> {
        let limit = if mode == "hang" { "3" } else { "30" };
        vec![
            ("CHROME", utf8(&self.bin)),
            ("FAKE_CHROME_MODE", mode),
            ("ROLLOUT_CHROME_TIMEOUT", limit),
        ]
    }

    /// Every argument Chrome was given, one per line.
    fn argv(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// The URLs the launcher pages went to, one per line.
    fn targets(&self) -> String {
        std::fs::read_to_string(&self.targets).unwrap_or_default()
    }
}

impl Drop for FakeChrome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The public host of prod as the Worker serves it: healthy without the
/// header.
fn prod_through_the_worker(req: &Req) -> Resp {
    prod(&req.through_worker())
}

/// The public host of dev: the same, behind the password.
fn dev_through_the_worker(req: &Req) -> Resp {
    dev(&req.through_worker())
}

/// T3 (search-front 017, ui-lib 063): through the Worker, a page that
/// hydrates on `/`, a map route and `/privacy` passes, and Chrome ran
/// headless, with its console on stderr, a virtual-time budget, its DOM
/// dumped, on the three pages.
#[test]
fn rollout_check_hydrate_passes_when_every_page_hydrates() {
    let fake = Fake::start(prod_through_the_worker);
    let chrome = FakeChrome::new("ok");
    let out = run(
        &check_script(),
        &["--via-worker", &fake.url()],
        &chrome.env("ok"),
    );
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("rollout-check: hydrate ok"), "{text}");
    let argv = chrome.argv();
    for flag in [
        "--headless=new",
        "--dump-dom",
        "--virtual-time-budget=15000",
        "--enable-logging=stderr",
    ] {
        assert_eq!(
            argv.lines().filter(|l| *l == flag).count(),
            3,
            "{flag}: {argv}"
        );
    }
    for page in HYDRATE_PAGES {
        let url = format!("{}{page}", fake.url());
        assert!(argv.lines().any(|l| l == url), "{url} visited: {argv}");
    }
}

/// Dev: Chrome passes the password gate through a launcher page (in the
/// script's private directory, gone when it exits) that opens the page with
/// the password in its URL in a popup (Chrome keeps it), then loads the
/// page without it (a page whose URL holds credentials cannot `fetch` its
/// .wasm); the password is on no command line and in no output.
#[test]
fn rollout_check_hydrate_passes_the_dev_gate_off_the_command_line() {
    let fake = Fake::start(dev_through_the_worker);
    let pass = password_file("hydrate-dev");
    let basic = STANDARD.encode(format!("deploy:{PASSWORD}"));
    let chrome = FakeChrome::new("dev");
    let url = fake.url();
    let out = run(
        &check_script(),
        &["--via-worker", "--password-file", utf8(&pass), &url],
        &chrome.env("ok"),
    );
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("rollout-check: hydrate ok"), "{text}");
    let host = url.trim_start_matches("http://");
    let wanted: Vec<String> = HYDRATE_PAGES
        .iter()
        .map(|page| format!("http://deploy:{PASSWORD}@{host}{page} -> {url}{page}"))
        .collect();
    assert_eq!(
        chrome.targets().lines().collect::<Vec<_>>(),
        wanted,
        "the launchers' targets"
    );
    let argv = chrome.argv();
    let launchers: Vec<&str> = argv.lines().filter(|l| l.starts_with("file://")).collect();
    assert_eq!(launchers.len(), 3, "{argv}");
    assert_eq!(
        argv.lines()
            .filter(|l| *l == "--disable-popup-blocking")
            .count(),
        3,
        "the launcher's popup is let through: {argv}"
    );
    for launcher in launchers {
        assert!(
            !Path::new(launcher.trim_start_matches("file://")).exists(),
            "{launcher} is left behind"
        );
    }
    for secret in [PASSWORD, basic.as_str()] {
        assert!(!argv.contains(secret), "on Chrome's command line: {argv}");
        assert!(!text.contains(secret), "printed: {text}");
    }
    let _ = std::fs::remove_file(pass);
}

/// The launcher's URL percent-encodes the user and the password: a
/// password with `@`, `:`, `/`, quotes, `%`, `<` or `\` neither breaks the
/// URL nor the page's script.
#[test]
fn rollout_check_hydrate_encodes_an_odd_password() {
    const ODD: &str = "p@ss:w/rd \"+'%<\\";
    let basic = STANDARD.encode(format!("deploy:{ODD}"));
    let want = format!("Basic {basic}");
    let fake = Fake::start(move |r| {
        if r.path != "/health" && r.header("authorization") != Some(want.as_str()) {
            return Resp::new(401, "")
                .with("www-authenticate", "Basic realm=\"PaliCanon viz dev\"");
        }
        prod_through_the_worker(r)
    });
    let pass = temp("password-header-odd");
    std::fs::write(&pass, format!("authorization: Basic {basic}\n")).expect("a password file");
    let chrome = FakeChrome::new("odd");
    let url = fake.url();
    let out = run(
        &check_script(),
        &["--via-worker", "--password-file", utf8(&pass), &url],
        &chrome.env("ok"),
    );
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let host = url.trim_start_matches("http://");
    let first = chrome
        .targets()
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(
        first,
        format!("http://deploy:p%40ss%3Aw%2Frd%20%22%2B%27%25%3C%5C@{host}/ -> {url}/")
    );
    let _ = std::fs::remove_file(pass);
}

/// A launcher whose popup never opened leaves Chrome on the launcher page:
/// a fail that says so, not a page that "never hydrated".
#[test]
fn rollout_check_hydrate_fails_when_chrome_stays_on_the_launcher() {
    let fake = Fake::start(dev_through_the_worker);
    let pass = password_file("hydrate-stuck");
    let chrome = FakeChrome::new("stuck");
    let out = run(
        &check_script(),
        &["--via-worker", "--password-file", utf8(&pass), &fake.url()],
        &chrome.env("stuck"),
    );
    assert_fails_naming(&out, "hydrate: /: Chrome never left the launcher", "stuck");
    let _ = std::fs::remove_file(pass);
}

/// A panic (or a wasm `RuntimeError`) in Chrome's console fails the
/// `hydrate` check, even when the marker is in the DOM; in dev, the line
/// printed has no password (Chrome names the page's URL in it).
#[test]
fn rollout_check_hydrate_fails_on_a_panic_in_the_browser() {
    let fake = Fake::start(prod_through_the_worker);
    for mode in ["panic", "runtime"] {
        let chrome = FakeChrome::new(mode);
        let out = run(
            &check_script(),
            &["--via-worker", &fake.url()],
            &chrome.env(mode),
        );
        assert_fails_naming(&out, "hydrate", mode);
        assert!(
            printed(&out).contains("panicked in the browser"),
            "{mode}: {}",
            printed(&out)
        );
    }
    let chrome = FakeChrome::new("privacy-panic");
    let out = run(
        &check_script(),
        &["--via-worker", &fake.url()],
        &chrome.env("privacy-panic"),
    );
    assert_fails_naming(
        &out,
        "hydrate: /privacy: the page panicked",
        "only /privacy panics",
    );
    let gated = Fake::start(dev_through_the_worker);
    let pass = password_file("hydrate-panic");
    let chrome = FakeChrome::new("dev-panic");
    let out = run(
        &check_script(),
        &["--via-worker", "--password-file", utf8(&pass), &gated.url()],
        &chrome.env("panic"),
    );
    assert_fails_naming(&out, "hydrate: /: the page panicked", "dev");
    assert!(!printed(&out).contains("deploy:"), "{}", printed(&out));
    let _ = std::fs::remove_file(pass);
}

/// A page whose dumped DOM has no `data-hydrated` never hydrated: a fail,
/// and so is a Chrome stopped by the time limit.
#[test]
fn rollout_check_hydrate_fails_without_the_marker_or_on_a_timeout() {
    let fake = Fake::start(prod_through_the_worker);
    for (mode, why) in [
        ("bare", "the page never hydrated"),
        ("hang", "Chrome stopped"),
    ] {
        let chrome = FakeChrome::new(mode);
        let started = Instant::now();
        let out = run(
            &check_script(),
            &["--via-worker", &fake.url()],
            &chrome.env(mode),
        );
        assert_fails_naming(&out, "hydrate", mode);
        assert!(
            printed(&out).contains(&format!("no data-hydrated on <html>: {why}")),
            "{mode}: {}",
            printed(&out)
        );
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "{mode}: the limit holds"
        );
    }
}

/// A Cloudflare challenge in the dumped page says nothing about the
/// service: exit 3 (a skip the orchestrator re-runs by hand), never 0.
#[test]
fn rollout_check_hydrate_tells_a_challenge_from_a_pass() {
    let fake = Fake::start(prod_through_the_worker);
    let chrome = FakeChrome::new("challenge");
    let out = run(
        &check_script(),
        &["--via-worker", &fake.url()],
        &chrome.env("challenge"),
    );
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(3), "{text}");
    assert!(text.contains("challenge"), "{text}");
    assert!(!text.contains("hydrate ok"), "{text}");
}

/// No usable Chrome is the runner's fault, not the service's: exit 2.
#[test]
fn rollout_check_hydrate_needs_a_chrome() {
    let fake = Fake::start(prod_through_the_worker);
    let out = run(
        &check_script(),
        &["--via-worker", &fake.url()],
        &[("CHROME", "/nonexistent/chrome")],
    );
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(text.contains("Chrome"), "{text}");
}

/// Chrome cannot send the origin header, so a revision URL (direct mode)
/// is never driven by it.
#[test]
fn rollout_check_hydrate_runs_through_the_worker_only() {
    let fake = Fake::start(prod);
    let chrome = FakeChrome::new("direct");
    let out = run(
        &check_script(),
        &[&fake.url()],
        &expect_release(&[
            ("ORIGIN_SECRET", SECRET),
            ("CHROME", utf8(&chrome.bin)),
            ("FAKE_CHROME_MODE", "panic"),
        ]),
    );
    let text = printed(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert_eq!(chrome.argv().len(), 0, "Chrome ran: {}", chrome.argv());
}

/// A directory holding a `curl` that logs its arguments, then runs the real
/// one: its path, the log, and a `PATH` that finds it first.
fn curl_shim() -> (PathBuf, PathBuf, String) {
    let dir = temp("curl-shim");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a shim dir");
    let real = Command::new("bash")
        .args(["-c", "command -v curl"])
        .output()
        .expect("bash runs");
    let real = String::from_utf8_lossy(&real.stdout).trim().to_owned();
    assert!(!real.is_empty(), "curl is installed");
    let log = dir.join("argv.log");
    let shim = dir.join("curl");
    let body = format!(
        "#!/bin/bash\nprintf '%s\\n' \"$@\" >> '{}'\nexec '{real}' \"$@\"\n",
        log.display()
    );
    std::fs::write(&shim, body).expect("a shim");
    Command::new("chmod")
        .args(["+x", utf8(&shim)])
        .status()
        .expect("chmod");
    let path = format!(
        "{}:{}",
        dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    (dir, log, path)
}

/// The secret and the password reach the server and nothing else: not the
/// output, not a curl argument, and the files' paths are not printed either.
#[test]
fn the_secrets_are_sent_but_never_printed_nor_put_on_a_command_line() {
    let (shim_dir, log, path) = curl_shim();
    let file = header_file("hygiene");
    let pass = password_file("hygiene");
    let basic = STANDARD.encode(format!("deploy:{PASSWORD}"));
    for (case, args, env) in [
        (
            "env",
            vec![],
            expect_release(&[("ORIGIN_SECRET", SECRET), ("PATH", path.as_str())]),
        ),
        (
            "file",
            vec![utf8(&file)],
            expect_release(&[("PATH", path.as_str())]),
        ),
    ] {
        let fake = Fake::start(dev);
        let url = fake.url();
        let mut all: Vec<&str> = vec!["--password-file", utf8(&pass), &url];
        all.extend(args);
        let out = run(&check_script(), &all, &env);
        let text = printed(&out);
        assert_eq!(out.status.code(), Some(0), "{case}: {text}");
        let (authed, admitted) = fake.authed_and_admitted();
        assert!(authed >= 3, "{case}: the secret reached the server");
        assert!(admitted >= 2, "{case}: the password reached the server");
        for shown in [SECRET, PASSWORD, basic.as_str(), utf8(&file), utf8(&pass)] {
            assert!(!text.contains(shown), "{case}: {shown} printed");
        }
        let argv = std::fs::read_to_string(&log).unwrap_or_default();
        assert!(!argv.is_empty(), "{case}: the shim saw curl run");
        for secret in [SECRET, PASSWORD, basic.as_str()] {
            assert!(!argv.contains(secret), "{case}: a secret on a command line");
        }
        let _ = std::fs::remove_file(&log);
    }
    let _ = std::fs::remove_file(file);
    let _ = std::fs::remove_file(pass);
    let _ = std::fs::remove_dir_all(shim_dir);
}
