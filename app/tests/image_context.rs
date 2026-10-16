//! The image build's context (task 005, slice S5): what `Dockerfile` copies
//! is enough for Cargo to load the workspace, and the build needs no
//! credential.
//!
//! From search-front/app/tests/image_context.rs. What differs: two
//! checkouts, not three (`vizualisation/`, `ui-lib/`: no API client); the
//! workspace has four members (`app`, `engine`, `layout`, `formats`), all
//! copied; no `token-pattern` beside `smoke.sh` (this service holds no API
//! token, so the image has no token check: `scripts/smoke.sh`'s header);
//! and tests of our own: the runtime env is the Leptos settings the brief
//! names, the image never sets `VIZ_GATE` (each deployment says it, the boot
//! refuses it unset), and the bundle is hash-named with `hash.txt` shipped
//! next to the binary (as learn-pali's image).
//!
//! No docker on the laptop, so the build context is simulated: the
//! Dockerfile's own `COPY` lines (the ones without `--from`) are replayed
//! from the two sibling checkouts (`vizualisation/`, `ui-lib/`, the folder
//! CI builds in) into a temp dir, then `cargo metadata --locked --offline`
//! loads the workspace there. A file the build needs but no `COPY` names
//! fails here, not on the runner.
//!
//! The property the credential-free build rests on: `Cargo.lock` holds no
//! git source. The private dependency is `[patch]`ed to the sibling
//! checkout and the lock records it as a path package, so Cargo has
//! nothing to fetch from GitHub.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository root (this test's package is `app/`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the repository root")
        .to_path_buf()
}

/// The Dockerfile's text.
fn dockerfile() -> String {
    fs::read_to_string(repo_root().join("Dockerfile")).expect("a Dockerfile at the repository root")
}

/// Whether a `Cargo.lock` records a package fetched from git.
fn lock_has_git_source(lock: &str) -> bool {
    lock.lines()
        .any(|line| line.trim_start().starts_with("source = \"git+"))
}

/// One instruction per entry: line continuations (`\`) joined, comments and
/// blank lines dropped. Heredoc bodies (`RUN <<EOF`) are not parsed; the
/// Dockerfile avoids them.
fn instructions(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for raw in text.lines() {
        let line = raw.trim();
        if current.is_empty() && (line.is_empty() || line.starts_with('#')) {
            continue;
        }
        if let Some(stripped) = line.strip_suffix('\\') {
            current.push_str(stripped.trim_end());
            current.push(' ');
        } else {
            current.push_str(line);
            out.push(std::mem::take(&mut current));
        }
    }
    out
}

/// `(sources, destination)` of a `COPY` that reads the build context.
fn context_copy(instruction: &str) -> Option<(Vec<String>, String)> {
    let rest = instruction.strip_prefix("COPY ")?;
    let words: Vec<&str> = rest.split_whitespace().collect();
    if words.iter().any(|w| w.starts_with("--from")) {
        return None;
    }
    let args: Vec<&str> = words.into_iter().filter(|w| !w.starts_with("--")).collect();
    let (dest, sources) = args.split_last()?;
    Some((
        sources.iter().map(ToString::to_string).collect(),
        (*dest).to_string(),
    ))
}

/// Copy a file or a directory tree.
fn copy_tree(from: &Path, to: &Path) {
    if from.is_dir() {
        fs::create_dir_all(to).expect("create a directory");
        for entry in fs::read_dir(from).expect("read a directory") {
            let entry = entry.expect("a directory entry");
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        }
    } else {
        fs::create_dir_all(to.parent().expect("a parent")).expect("create a parent");
        fs::copy(from, to).unwrap_or_else(|e| panic!("copy {}: {e}", from.display()));
    }
}

/// The build context folder: the one holding `vizualisation/` and
/// `ui-lib/`. This checkout stands for `vizualisation/` (a worktree has
/// another name), the sibling is beside it.
fn context_source(name: &str) -> PathBuf {
    let root = repo_root();
    match name {
        "vizualisation" => root,
        other => root.join("..").join(other),
    }
}

/// Resolve a `COPY` source (`ui-lib/src`, `vizualisation/app`) in the real tree.
fn resolve_source(source: &str) -> PathBuf {
    let (first, rest) = source.split_once('/').unwrap_or((source, ""));
    context_source(first).join(rest)
}

/// One `COPY` of the context into the stage's file system `dir`.
fn replay_copy(dir: &Path, workdir: &Path, sources: &[String], dest: &str) {
    let dest_abs = workdir.join(dest);
    let inside = dir.join(dest_abs.strip_prefix("/").expect("an absolute destination"));
    for source in sources {
        let from = resolve_source(source);
        assert!(
            from.exists(),
            "COPY {source}: not in the sibling checkouts ({})",
            from.display()
        );
        let target = if dest.ends_with('/') {
            inside.join(Path::new(source).file_name().expect("a file name"))
        } else {
            inside.clone()
        };
        copy_tree(&from, &target);
    }
}

/// Replay the Dockerfile's `COPY` lines of the `builder` stage into `dir`
/// (the stage's file system), following `WORKDIR`.
fn replay_builder(dir: &Path) {
    let mut stage = String::new();
    let mut workdir = PathBuf::from("/");
    for instruction in instructions(&dockerfile()) {
        if let Some(from) = instruction.strip_prefix("FROM ") {
            stage = from
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .to_string();
            workdir = PathBuf::from("/");
        } else if let Some(wd) = instruction.strip_prefix("WORKDIR ") {
            workdir = workdir.join(wd.trim());
        } else if stage == "builder"
            && let Some((sources, dest)) = context_copy(&instruction)
        {
            replay_copy(dir, &workdir, &sources, &dest);
        }
    }
}

fn siblings_present() -> bool {
    context_source("ui-lib").is_dir()
}

#[test]
fn the_lock_holds_no_git_source() {
    let lock = fs::read_to_string(repo_root().join("Cargo.lock")).expect("Cargo.lock");
    assert!(
        !lock_has_git_source(&lock),
        "Cargo.lock records a git source: the image build would need a GitHub credential (search-front 007, R1)"
    );
}

#[test]
fn a_git_source_in_a_lock_is_seen() {
    let lock = "[[package]]\nname = \"x\"\nversion = \"1.0.0\"\nsource = \"git+ssh://git@github.com/PaliCanon/ui-lib.git?branch=main#abc\"\n";
    assert!(lock_has_git_source(lock));
    assert!(lock_has_git_source(
        "  source = \"git+https://github.com/a/b\""
    ));
    let registry = "source = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
    assert!(!lock_has_git_source(registry));
}

#[test]
fn the_copied_files_are_enough_for_cargo_to_load_the_workspace() {
    if !siblings_present() {
        eprintln!("skipped: ../ui-lib is not beside this checkout (CI checks it out)");
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "vizualisation-image-context-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    replay_builder(&dir);
    let manifest = dir.join("src/vizualisation/Cargo.toml");
    assert!(
        manifest.is_file(),
        "the builder stage copies no vizualisation/Cargo.toml under /src/vizualisation"
    );
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    // `cargo metadata` resolves every target (windows, UEFI: `r-efi`, ...),
    // while a runner that only built and tested for its host has fetched the
    // host's crates (search-front 007 F2). Fetch what the lock needs for every target
    // first, as the image build's own network step does, so the `--offline`
    // load below judges the copied files and not the state of `CARGO_HOME`.
    // No git source exists (the test above), so no credential is involved.
    let fetch = Command::new(&cargo)
        .args(["fetch", "--locked", "--manifest-path"])
        .arg(&manifest)
        .env("GIT_SSH_COMMAND", "/bin/false")
        .output()
        .expect("run cargo fetch");
    assert!(
        fetch.status.success(),
        "cargo fetch on the replayed context failed:\n{}",
        String::from_utf8_lossy(&fetch.stderr)
    );
    let output = Command::new(&cargo)
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&manifest)
        .env("GIT_SSH_COMMAND", "/bin/false")
        .output()
        .expect("run cargo metadata");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = fs::remove_dir_all(&dir);
    assert!(
        output.status.success(),
        "cargo metadata on the replayed context failed:\n{stderr}"
    );
}

/// The relative paths the build reads outside Cargo's view: the `@import` and
/// `@source` of `app/style/main.css` (Tailwind) and the `#[path]` of
/// `app/build.rs`. Cargo's metadata does not follow them; the build does.
fn relative_reads(file: &Path) -> Vec<PathBuf> {
    let text = fs::read_to_string(file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
    let base = file.parent().expect("a directory");
    let mut reads = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let quoted = if line.starts_with("@import")
            || line.starts_with("@source")
            || line.starts_with("#[path")
        {
            line.split('"').nth(1)
        } else {
            None
        };
        if let Some(path) = quoted.filter(|p| p.starts_with("..")) {
            reads.push(base.join(path));
        }
    }
    reads
}

#[test]
fn the_files_the_build_reads_outside_cargo_are_copied() {
    if !siblings_present() {
        eprintln!("skipped: ../ui-lib is not beside this checkout (CI checks it out)");
        return;
    }
    let dir =
        std::env::temp_dir().join(format!("vizualisation-image-reads-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    replay_builder(&dir);
    let app = dir.join("src/vizualisation/app");
    let mut missing = Vec::new();
    let mut seen = 0;
    for file in [app.join("style/main.css"), app.join("build.rs")] {
        for read in relative_reads(&file) {
            seen += 1;
            if !read.exists() {
                missing.push(format!(
                    "{} reads {}",
                    file.strip_prefix(&dir).unwrap().display(),
                    read.strip_prefix(&dir).unwrap_or(&read).display()
                ));
            }
        }
    }
    let _ = fs::remove_dir_all(&dir);
    assert!(
        seen >= 3,
        "expected the theme import, the source scan and the placeholders path, saw {seen}"
    );
    assert!(
        missing.is_empty(),
        "the image does not copy what the build reads: {missing:?}"
    );
}

#[test]
fn the_image_runs_in_production_mode_as_a_non_root_user() {
    let text = dockerfile();
    let all = instructions(&text);
    assert!(
        all.iter().any(|i| i.contains("LEPTOS_ENV=PROD")),
        "the image sets LEPTOS_ENV=PROD"
    );
    let user = all
        .iter()
        .rev()
        .find_map(|i| i.strip_prefix("USER "))
        .expect("a USER instruction");
    assert!(
        user.trim() != "root" && user.trim() != "0",
        "the image must not run as root, got USER {user}"
    );
    assert!(
        all.iter().any(|i| i.starts_with("CMD ")),
        "a CMD starts the server"
    );
}

/// The data files `app/src/data.rs` reads at boot (design v4: baked into the
/// image, loaded at boot).
const DATA_FILES: [&str; 6] = [
    "graph.json",
    "graph-sim.json",
    "graph-sim.bin",
    "translations.json",
    "blurbs-en.json",
    "blurbs-fr.json",
];

#[test]
fn the_image_carries_the_data_and_points_the_service_at_it() {
    let text = dockerfile();
    let all = instructions(&text);
    let runtime = all
        .iter()
        .rposition(|i| i.starts_with("FROM "))
        .expect("a runtime stage");
    let copies: Vec<&String> = all[runtime..]
        .iter()
        .filter(|i| i.starts_with("COPY ") && !i.contains("--from"))
        .collect();
    for file in DATA_FILES {
        assert!(
            repo_root().join("public").join(file).is_file(),
            "public/{file} is committed"
        );
        let source = format!("vizualisation/public/{file}");
        assert!(
            copies
                .iter()
                .any(|c| c.split_whitespace().any(|w| w == source)
                    && c.trim_end().ends_with("./data/")),
            "the runtime stage copies {source} into ./data/: {copies:?}"
        );
    }
    assert!(
        all[runtime..]
            .iter()
            .any(|i| i.contains("VIZ_DATA_DIR=data")),
        "the image sets VIZ_DATA_DIR=data"
    );
}

#[test]
fn the_dockerfile_holds_no_credential() {
    let text = dockerfile();
    let all = instructions(&text);
    let sensitive = ["TOKEN", "SECRET", "PASSWORD", "KEY", "CREDENTIAL", "AUTH"];
    for instruction in &all {
        if let Some(rest) = instruction
            .strip_prefix("ARG ")
            .or_else(|| instruction.strip_prefix("ENV "))
        {
            let upper = rest.to_uppercase();
            let name = upper.split(['=', ' ']).next().unwrap_or_default();
            assert!(
                !sensitive.iter().any(|s| name.contains(s)),
                "an ARG/ENV named like a credential: {instruction}"
            );
        }
        assert!(
            !instruction.contains("type=secret"),
            "a BuildKit secret mount: {instruction}"
        );
        assert!(
            !instruction.contains("type=ssh"),
            "an ssh mount: {instruction}"
        );
        assert!(
            !instruction.contains("insteadOf"),
            "a git URL rewrite: {instruction}"
        );
        assert!(
            !instruction.contains("x-access-token"),
            "a token in a URL: {instruction}"
        );
        assert!(
            !instruction.contains("GIT_SSH_COMMAND"),
            "a git transport override: {instruction}"
        );
    }
}

#[test]
fn the_build_fails_when_the_lock_gains_a_git_source() {
    let text = dockerfile();
    let guarded = instructions(&text)
        .iter()
        .any(|i| i.starts_with("RUN ") && i.contains("git+") && i.contains("Cargo.lock"));
    assert!(
        guarded,
        "a RUN must fail the build when Cargo.lock holds a git source"
    );
}

#[test]
fn the_server_runs_the_smoke_test_with_its_own_throwaway_secret() {
    let all = instructions(&dockerfile());
    let smoke = all
        .iter()
        .rposition(|i| i.starts_with("RUN ") && i.contains("smoke.sh"));
    let last_run = all.iter().rposition(|i| i.starts_with("RUN "));
    assert!(smoke.is_some(), "a RUN runs scripts/smoke.sh");
    assert_eq!(smoke, last_run, "the smoke test is the last RUN");
}

/// `scripts/smoke.sh BINARY` makes its own throwaway origin secret and dev
/// password and a free port: the call is exactly `RUN ./smoke.sh
/// ./vizualisation`, with no `ORIGIN_SECRET=` or `DEV_PASSWORD=` prefix (the
/// script would overwrite them anyway).
#[test]
fn the_smoke_call_has_the_form_the_script_takes() {
    let all = instructions(&dockerfile());
    let smoke = all
        .iter()
        .rfind(|i| i.starts_with("RUN ") && i.contains("smoke.sh"))
        .expect("a RUN runs smoke.sh");
    assert_eq!(
        smoke, "RUN ./smoke.sh ./vizualisation",
        "smoke.sh takes the binary and nothing else: no env prefix, no --port"
    );
}

/// The badge reads the branch and the commit at compile time (`app/build.rs`);
/// the image has no `.git`, so the builder stage takes them as build args and
/// exports them to the environment of `cargo leptos build`.
#[test]
fn the_builder_exports_the_build_branch_and_sha_before_the_build() {
    let mut stage = String::new();
    let mut position = std::collections::HashMap::new();
    for (index, instruction) in instructions(&dockerfile()).iter().enumerate() {
        if let Some(from) = instruction.strip_prefix("FROM ") {
            stage = from
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .to_string();
        } else if stage == "builder" {
            for name in ["BUILD_BRANCH", "BUILD_SHA"] {
                if instruction == &format!("ARG {name}") {
                    position.insert(format!("ARG {name}"), index);
                }
                if instruction.starts_with("ENV ")
                    && instruction.contains(&format!("{name}=${name}"))
                {
                    position.insert(format!("ENV {name}"), index);
                }
            }
            if instruction.starts_with("RUN ") && instruction.contains("cargo leptos build") {
                position.insert("build".to_string(), index);
            }
        }
    }
    let build = *position
        .get("build")
        .expect("the builder runs cargo leptos build");
    for name in ["BUILD_BRANCH", "BUILD_SHA"] {
        let arg = position.get(&format!("ARG {name}"));
        let env = position.get(&format!("ENV {name}"));
        assert!(arg.is_some(), "the builder stage has no `ARG {name}`");
        assert!(
            env.is_some(),
            "the builder stage has no `ENV {name}=${name}`"
        );
        assert!(
            arg < env && *env.unwrap() < build,
            "ARG {name}, then ENV {name}, then cargo leptos build"
        );
    }
}

/// The runtime settings the brief names (`get_configuration(None)` reads
/// them; the image has no Cargo.toml): the output name the page links its
/// bundle by, the site root and pkg dir, the address and port Cloud Run
/// sends its traffic to, and `LEPTOS_ENV=PROD` (the boot then requires
/// `ORIGIN_SECRET`).
#[test]
fn the_image_sets_the_leptos_runtime_env() {
    let all = instructions(&dockerfile());
    let env: Vec<&String> = all.iter().filter(|i| i.starts_with("ENV ")).collect();
    for setting in [
        "LEPTOS_OUTPUT_NAME=vizualisation",
        "LEPTOS_SITE_ROOT=site",
        "LEPTOS_SITE_PKG_DIR=pkg",
        "LEPTOS_SITE_ADDR=0.0.0.0:8080",
        "LEPTOS_ENV=PROD",
        "PORT=8080",
    ] {
        assert!(
            env.iter()
                .any(|i| i.split_whitespace().any(|word| word == setting)),
            "no ENV sets {setting}"
        );
    }
}

/// `VIZ_GATE` is said by each deployment (infra: `password` on dev, `off` on
/// prod) and the boot refuses to start without it (exit 2). A default in the
/// image would turn a forgotten env into a silent choice: no `ENV` or `ARG`
/// of the Dockerfile names it, nor the dev password.
#[test]
fn the_image_never_chooses_the_gate() {
    for instruction in instructions(&dockerfile()) {
        if instruction.starts_with("ENV ") || instruction.starts_with("ARG ") {
            assert!(
                !instruction.contains("VIZ_GATE") && !instruction.contains("DEV_PASSWORD"),
                "the image must not set the gate or its password: {instruction}"
            );
        }
    }
}

/// The bundle is hash-named (learn-pali/Cargo.toml: `hash-files = true`):
/// `cargo leptos build` names `pkg/vizualisation.<hash>.{js,wasm,css}` and
/// writes `hash.txt` next to the server binary, so a new build is never
/// served from a browser's or the CDN's cache of the old one.
#[test]
fn the_workspace_hash_names_the_bundle() {
    let manifest = fs::read_to_string(repo_root().join("Cargo.toml")).expect("the root Cargo.toml");
    let section = manifest
        .split("[[workspace.metadata.leptos]]")
        .nth(1)
        .expect("a [[workspace.metadata.leptos]] section");
    let section = section.split("\n[").next().unwrap_or_default();
    assert!(
        section
            .lines()
            .any(|line| line.trim() == "hash-files = true"),
        "[[workspace.metadata.leptos]] sets no `hash-files = true`"
    );
}

/// The server reads `hash.txt` NEXT TO ITS OWN EXECUTABLE to name the hashed
/// files, and only when `LEPTOS_HASH_FILES=true` (the image has no
/// Cargo.toml): the builder keeps the file the build wrote, the runtime
/// stage puts it beside `./vizualisation` (learn-pali/app/Dockerfile).
#[test]
fn the_image_ships_hash_txt_next_to_the_binary() {
    let all = instructions(&dockerfile());
    let build = all
        .iter()
        .find(|i| i.starts_with("RUN ") && i.contains("cargo leptos build"))
        .expect("the builder runs cargo leptos build");
    assert!(
        build.contains("cp target/release/vizualisation target/release/hash.txt /out/"),
        "the builder does not copy target/release/hash.txt to /out: {build}"
    );
    let runtime = all
        .iter()
        .rposition(|i| i.starts_with("FROM "))
        .expect("a runtime stage");
    let beside = all[runtime..].iter().any(|i| {
        let words: Vec<&str> = i.split_whitespace().collect();
        i.starts_with("COPY --from=builder")
            && words.contains(&"/out/vizualisation")
            && words.contains(&"/out/hash.txt")
            && words.last() == Some(&"./")
    });
    assert!(
        beside,
        "the runtime stage does not copy /out/hash.txt next to ./vizualisation"
    );
    assert!(
        all[runtime..].iter().any(|i| i.starts_with("ENV ")
            && i.split_whitespace().any(|w| w == "LEPTOS_HASH_FILES=true")),
        "the runtime stage sets no LEPTOS_HASH_FILES=true"
    );
}
