//! `scripts/smoke.sh` and `scripts/rollout-check.sh` as processes (task 005,
//! slice S5): against the real binary and against a fake server that plays
//! every way a deployment can be wrong. A check that passes only on the
//! happy path proves nothing, so each check has a server that must fail it,
//! and the failure names the check. The origin secret and the dev password
//! are in no output and no argument.
//!
//! From search-front/app/tests/site_scripts.rs. What differs: the checks of
//! this service (design v4): `/healthz` behind the origin guard only, the
//! password gate of dev (401 with `WWW-Authenticate` without the password,
//! the pages and the files behind it; prod has no `WWW-Authenticate`), the
//! boot that refuses an unset `VIZ_GATE` (exit 2). No API, so no live search
//! and no token shape: the `search` and `token` checks and the
//! `token-pattern` file are gone; `smoke.sh` checks instead that no
//! response echoes its throwaway secret or password (`leak`).
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

/// The page the real service renders, as far as the scripts read it.
fn page(extra: &str) -> String {
    format!(
        "<!DOCTYPE html><html><head><title>Suttas Map — PaliCanon</title>\
         <link rel=\"modulepreload\" href=\"/pkg/vizualisation.js\">\
         <link rel=\"preload\" href=\"{WASM}\" as=\"fetch\" type=\"application/wasm\">\
         </head><body>{extra}</body></html>"
    )
}

/// A healthy deployment, in the design's order: the origin guard on every
/// path (403, empty body), `/healthz` behind it alone, then the password
/// gate when `gated` (401 asking for it), then the page and the wasm.
fn serve(req: &Req, gated: bool) -> Resp {
    if !req.authed() {
        return Resp::new(403, "").with("cache-control", "no-store");
    }
    if req.path == "/healthz" {
        return Resp::new(200, "ok 63bdba3\n").with("content-type", "text/plain");
    }
    if gated && !req.admitted() {
        return Resp::new(401, "")
            .with("www-authenticate", "Basic realm=\"PaliCanon viz dev\"")
            .with("cache-control", "private, no-store");
    }
    if req.path == WASM {
        return Resp::new(200, "\0asm").with("content-type", "application/wasm");
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
/// `/pkg/vizualisation.wasm`.
fn site_root(name: &str, with_wasm: bool) -> PathBuf {
    let dir = temp(&format!("site-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pkg")).expect("a site root");
    if with_wasm {
        for file in ["pkg/vizualisation_bg.wasm", "pkg/vizualisation.wasm"] {
            std::fs::write(dir.join(file), b"\0asm\x01\0\0\0").expect("wasm");
        }
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
        .env("SMOKE_WAIT", "8");
    for (name, value) in env {
        command.env(name, value);
    }
    command.output().expect("bash runs the script")
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

/// A dev server whose `/healthz`, to a caller with the secret, answers
/// `status` and `body`.
fn health(status: u16, body: &str) -> Handler {
    let body = body.to_owned();
    Box::new(move |r| {
        if r.authed() && r.path == "/healthz" {
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

/// `/healthz` and the origin guard.
fn origin_cases() -> Vec<Case> {
    vec![
        ("healthz", "/healthz is a 500", health(500, "boom")),
        (
            "healthz",
            "/healthz does not say ok",
            health(200, "starting"),
        ),
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
            "/healthz without the header answers 200",
            Box::new(|r| {
                if r.path == "/healthz" {
                    Resp::new(200, "ok 63bdba3\n")
                } else {
                    dev(r)
                }
            }),
        ),
    ]
}

/// The password gate, the page and the wasm.
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
                if r.authed() && r.path != "/healthz" && !r.admitted() {
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
        "healthz ok",
        "guard ok",
        "gate ok",
        "root ok",
        "wasm ok",
        "leak ok",
        "boot ok",
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

/// What only `smoke.sh` checks: the wasm's type, and no echo of a secret.
fn smoke_only_cases() -> Vec<Case> {
    vec![
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
        &[("ORIGIN_SECRET", SECRET)],
    );
    let text = printed(&out);
    assert_eq!(
        out.status.code(),
        Some(0),
        "dev, secret from the env: {text}"
    );
    for check in ["healthz ok", "gate ok", "root ok", "wasm ok", "guard ok"] {
        assert!(text.contains(check), "{check} is reported: {text}");
    }
    let out = run(
        &check_script(),
        &["--password-file", utf8(&pass), &url, utf8(&file)],
        &[],
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
    let out = run(&check_script(), &[&url, utf8(&file)], &[]);
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
    let out = run(&check_script(), &[&prod_service.url(), utf8(&file)], &[]);
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
        let out = run(&check_script(), &all, &[]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "via the Worker, gate {gate}: {}",
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
            &[("ORIGIN_SECRET", SECRET)],
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
        &[("ORIGIN_SECRET", SECRET)],
    );
    assert_eq!(out.status.code(), Some(0), "prod: {}", printed(&out));
    let asking =
        Fake::start(|r| prod(r).with("www-authenticate", "Basic realm=\"PaliCanon viz dev\""));
    let out = run(
        &check_script(),
        &[&asking.url()],
        &[("ORIGIN_SECRET", SECRET)],
    );
    assert_fails_naming(&out, "gate", "prod asks for a password");
    let gated = Fake::start(dev);
    let out = run(
        &check_script(),
        &[&gated.url()],
        &[("ORIGIN_SECRET", SECRET)],
    );
    assert_fails_naming(&out, "root", "prod behind the gate");
}

#[test]
fn rollout_check_demands_a_secret_and_refuses_to_send_it_to_the_worker() {
    let fake = Fake::start(dev);
    let out = run(&check_script(), &[&fake.url()], &[]);
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
    let out = run(&check_script(), &[&fake.url(), utf8(&bad)], &[]);
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
            &[("ORIGIN_SECRET", SECRET)],
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
/// first request is `/healthz`, so that is the check a relayed failure names.
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
    assert_fails_naming(&out, "healthz", "the service's own 500, relayed");
    let refusal = Fake::start(|_| Resp::new(403, "forbidden"));
    let out = run(&check_script(), &["--via-worker", &refusal.url()], &[]);
    assert_fails_naming(&out, "healthz", "a 403");
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
        assert_fails_naming(&out, "healthz", &format!("the service's own 500 {body:?}"));
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
        &[("ORIGIN_SECRET", SECRET)],
    );
    assert_fails_naming(&out, "healthz", "direct mode");
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
            vec![("ORIGIN_SECRET", SECRET), ("PATH", path.as_str())],
        ),
        ("file", vec![utf8(&file)], vec![("PATH", path.as_str())]),
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
