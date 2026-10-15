//! The service as a process: the binary booted with an environment, the
//! way Cloud Run and `cargo leptos watch` run it (task 005, slice S5). It
//! is the only place where the real `main`, its exit code and every line it
//! prints can be read; the boot's rules and the router's layers are tested
//! in-process (`boot.rs`, `router.rs`).
//!
//! From search-front/app/tests/boot_process.rs. What differs: this boot's
//! variables (`VIZ_GATE`, `DEV_PASSWORD`, `ORIGIN_SECRET`; no API, so no
//! mock and no token), the exit code pinned to 2 (the smoke test and the
//! design rely on it), and the gate's answers (401 without the password,
//! `/healthz` behind the origin guard only).
#![cfg(feature = "ssr")]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// A dev password no real one looks like: it must be in no line printed.
const PASSWORD_CANARY: &str = "gate_canary_process_41c9";
/// An origin secret no real one looks like: it must be in no line printed.
const ORIGIN_CANARY: &str = "origin_canary_process_b7d3";

/// The running binary, killed when dropped (a failed assertion leaves none).
struct Service(Option<Child>);

impl Drop for Service {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// A free local port, never one this test binary already handed out: the
/// tests run in parallel and a port is released before its service binds it,
/// so two of them could otherwise be given the same one.
fn free_port() -> u16 {
    static HANDED_OUT: Mutex<Vec<u16>> = Mutex::new(Vec::new());
    loop {
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("a free port")
            .local_addr()
            .expect("an address")
            .port();
        let mut handed_out = HANDED_OUT.lock().expect("ports lock");
        if !handed_out.contains(&port) {
            handed_out.push(port);
            return port;
        }
    }
}

/// The binary, with the given variables and its own address; none of the
/// boot's variables is inherited from the test's environment.
fn command(port: u16, env: &[(&str, &str)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vizualisation"));
    command
        .env_remove("VIZ_GATE")
        .env_remove("DEV_PASSWORD")
        .env_remove("ORIGIN_SECRET")
        .env_remove("LEPTOS_ENV")
        .env_remove("K_SERVICE")
        .env("LEPTOS_OUTPUT_NAME", "vizualisation")
        .env(
            "VIZ_DATA_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../public"),
        )
        .env("LEPTOS_SITE_ADDR", format!("127.0.0.1:{port}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in env {
        command.env(name, value);
    }
    command
}

/// What the service printed once stopped.
struct Ended {
    stdout: String,
    stderr: String,
}

/// Stops the service (if it still runs) and reads everything it printed.
fn finish(mut service: Service) -> Ended {
    let mut child = service.0.take().expect("a running service");
    let _ = child.kill();
    let out = child.wait_with_output().expect("the service's output");
    Ended {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Starts the service and waits until it accepts connections.
fn start(port: u16, env: &[(&str, &str)]) -> Service {
    let child = command(port, env).spawn().expect("the binary starts");
    let mut service = Service(Some(child));
    let deadline = Instant::now() + Duration::from_secs(30);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        let child = service.0.as_mut().expect("a running service");
        assert!(
            child.try_wait().expect("a status").is_none(),
            "the service stopped before listening"
        );
        assert!(Instant::now() < deadline, "the service never listened");
        std::thread::sleep(Duration::from_millis(50));
    }
    service
}

/// `GET path` with extra header lines (each ending in CRLF), over a fresh
/// HTTP/1.0 connection: the status and the whole response.
fn get_with(port: u16, path: &str, headers: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .expect("a timeout");
    write!(
        stream,
        "GET {path} HTTP/1.0\r\nHost: localhost\r\n{headers}\r\n"
    )
    .expect("write");
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw).into_owned();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("a status line");
    (status, text)
}

/// Runs the command for at most 10 s: its output, and whether it ended by
/// itself (a service that listens is killed, and did not).
fn run_to_exit(mut command: Command) -> (std::process::Output, bool) {
    let mut child = command.spawn().expect("the binary starts");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut by_itself = true;
    while child.try_wait().expect("a status").is_none() {
        if Instant::now() > deadline {
            by_itself = false;
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    (child.wait_with_output().expect("the output"), by_itself)
}

/// Runs a build that must refuse to start: exit 2 before listening, stderr
/// names `named`, no canary is printed.
fn assert_refuses(env: &[(&str, &str)], named: &str) {
    let (out, ended_by_itself) = run_to_exit(command(free_port(), env));
    assert!(ended_by_itself, "{env:?} must stop at boot, not listen");
    assert_eq!(out.status.code(), Some(2), "{env:?} must exit 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(named), "stderr names {named}: {stderr}");
    let everything = format!("{stderr}{}", String::from_utf8_lossy(&out.stdout));
    for canary in [PASSWORD_CANARY, ORIGIN_CANARY] {
        assert!(!everything.contains(canary), "leaked: {everything}");
    }
}

#[test]
fn an_unset_or_half_configured_gate_refuses_to_start_and_names_the_variable() {
    assert_refuses(&[], "VIZ_GATE");
    assert_refuses(&[("ORIGIN_SECRET", ORIGIN_CANARY)], "VIZ_GATE");
    assert_refuses(&[("VIZ_GATE", "maybe")], "VIZ_GATE");
    assert_refuses(&[("VIZ_GATE", " ")], "VIZ_GATE");
    assert_refuses(
        &[("VIZ_GATE", "password"), ("ORIGIN_SECRET", ORIGIN_CANARY)],
        "DEV_PASSWORD",
    );
    assert_refuses(
        &[("VIZ_GATE", "password"), ("DEV_PASSWORD", "  ")],
        "DEV_PASSWORD",
    );
}

#[test]
fn missing_data_refuses_to_start_and_names_the_file() {
    let empty = std::env::temp_dir().join(format!("viz-no-data-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    let dir = empty.to_str().expect("utf-8");
    assert_refuses(
        &[("VIZ_GATE", "off"), ("VIZ_DATA_DIR", dir)],
        "graph-sim.json",
    );
}

#[test]
fn a_deployed_build_without_the_origin_secret_refuses_to_start() {
    let dev = [("VIZ_GATE", "password"), ("DEV_PASSWORD", PASSWORD_CANARY)];
    for deployed in [("LEPTOS_ENV", "PROD"), ("K_SERVICE", "vizualisation-dev")] {
        let mut env = dev.to_vec();
        env.push(deployed);
        assert_refuses(&env, "ORIGIN_SECRET");
    }
    let mut blank = dev.to_vec();
    blank.push(("ORIGIN_SECRET", "  "));
    assert_refuses(&blank, "ORIGIN_SECRET");
}

/// The requests dev is asked, in the layers' order: (path, header lines,
/// status wanted). The origin guard first, `/healthz` behind it alone, then
/// the password.
fn dev_cases() -> Vec<(&'static str, String, u16)> {
    let origin = format!("x-origin-auth: {ORIGIN_CANARY}\r\n");
    let basic = |password: &str| {
        format!(
            "authorization: Basic {}\r\n",
            STANDARD.encode(format!("anyone:{password}"))
        )
    };
    vec![
        ("/", String::new(), 403),
        ("/", basic(PASSWORD_CANARY), 403),
        ("/", "x-origin-auth: nope\r\n".to_owned(), 403),
        ("/healthz", String::new(), 403),
        ("/healthz", origin.clone(), 200),
        ("/", origin.clone(), 401),
        ("/", format!("{origin}{}", basic("nope")), 401),
        ("/", format!("{origin}{}", basic(PASSWORD_CANARY)), 200),
    ]
}

#[test]
fn dev_answers_the_origin_then_the_password_and_logs_each_once() {
    let port = free_port();
    let service = start(
        port,
        &[
            ("VIZ_GATE", "password"),
            ("DEV_PASSWORD", PASSWORD_CANARY),
            ("ORIGIN_SECRET", ORIGIN_CANARY),
            ("LEPTOS_ENV", "PROD"),
        ],
    );
    let cases = dev_cases();
    for (path, headers, want) in cases {
        let (status, text) = get_with(port, path, &headers);
        assert_eq!(status, want, "{path} with {headers:?}: {text}");
        if status == 401 {
            assert!(
                text.to_ascii_lowercase()
                    .contains("www-authenticate: basic"),
                "a 401 asks for the password: {text}"
            );
        }
        for canary in [PASSWORD_CANARY, ORIGIN_CANARY] {
            assert!(!text.contains(canary), "{path}: a secret in the response");
        }
    }
    let ended = finish(service);
    for line in ["gate: password", "origin: checked"] {
        assert_eq!(ended.stdout.matches(line).count(), 1, "{}", ended.stdout);
    }
    for (what, text) in [("stdout", &ended.stdout), ("stderr", &ended.stderr)] {
        assert!(!text.contains(PASSWORD_CANARY), "password in {what}");
        assert!(!text.contains(ORIGIN_CANARY), "origin secret in {what}");
    }
}

#[test]
fn prod_has_no_gate_and_asks_for_no_password() {
    let port = free_port();
    let service = start(
        port,
        &[
            ("VIZ_GATE", "off"),
            ("ORIGIN_SECRET", ORIGIN_CANARY),
            ("LEPTOS_ENV", "PROD"),
        ],
    );
    let origin = format!("x-origin-auth: {ORIGIN_CANARY}\r\n");
    let (status, text) = get_with(port, "/", &origin);
    assert_eq!(status, 200, "{text}");
    assert!(
        !text.to_ascii_lowercase().contains("www-authenticate"),
        "{text}"
    );
    let (status, _) = get_with(port, "/", "");
    assert_eq!(status, 403, "the origin guard is still on");
    let ended = finish(service);
    assert_eq!(
        ended.stdout.matches("gate: off").count(),
        1,
        "{}",
        ended.stdout
    );
}

#[test]
fn a_local_run_says_the_origin_is_open() {
    let port = free_port();
    let service = start(port, &[("VIZ_GATE", "off")]);
    let (status, _) = get_with(port, "/", "");
    assert_eq!(status, 200);
    let ended = finish(service);
    assert_eq!(
        ended.stdout.matches("origin: open (local)").count(),
        1,
        "{}",
        ended.stdout
    );
}
