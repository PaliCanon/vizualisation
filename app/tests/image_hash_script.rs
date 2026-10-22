//! `scripts/image-hash.sh` (task 005, slice S5): the image tag is a hash of
//! what shapes the image, computed once in CI after the sibling checkout.
//! Two temp git repositories stand for `vizualisation/` and `ui-lib/`.
//!
//! From search-front/app/tests/image_hash_script.rs. What differs: one
//! sibling (ui-lib; this service has no API client), and the inputs hold
//! the workspace's other members, `engine`, `layout`, `formats` and
//! `i18n-check`, which the Dockerfile copies too.
//!
//! The inputs are the tracked files of `app`, `engine`, `layout`, `formats`, `i18n-check`,
//! `Cargo.toml`, `Cargo.lock`, `Dockerfile` and `scripts` (the Dockerfile's
//! text and the smoke test are in the hash, so a fix to either changes the
//! tag), plus the HEAD of `ui-lib`. Nothing else: `infra/`, `.task/`, the
//! README, the `LICENSE`, the rest of `public/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A temp directory holding the two repositories, removed on drop.
struct Workspace {
    dir: PathBuf,
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.org"])
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn write(repo: &Path, file: &str, text: &str) {
    let path = repo.join(file);
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
    fs::write(path, text).expect("write a file");
}

/// Commit everything in `repo` (the test repositories only).
fn commit(repo: &Path, message: &str) {
    git(repo, &["add", "."]);
    git(repo, &["commit", "-q", "-m", message]);
}

impl Workspace {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("image-hash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let viz = dir.join("vizualisation");
        for repo in ["vizualisation", "ui-lib"] {
            fs::create_dir_all(dir.join(repo)).expect("create a repository");
            git(&dir.join(repo), &["init", "-q", "-b", "main"]);
        }
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../scripts/image-hash.sh");
        let script = fs::read_to_string(&real).expect("scripts/image-hash.sh");
        write(&viz, "scripts/image-hash.sh", &script);
        write(&viz, "scripts/smoke.sh", "echo smoke\n");
        write(&viz, "app/src/lib.rs", "pub fn a() {}\n");
        write(&viz, "app/Cargo.toml", "[package]\nname = \"a\"\n");
        write(&viz, "engine/src/lib.rs", "// engine\n");
        write(&viz, "layout/src/lib.rs", "// layout\n");
        write(&viz, "formats/src/lib.rs", "// formats\n");
        write(&viz, "i18n-check/src/lib.rs", "// i18n-check\n");
        write(&viz, "Cargo.toml", "[workspace]\n");
        write(&viz, "Cargo.lock", "# lock\n");
        write(&viz, "Dockerfile", "FROM scratch\n");
        write(&viz, "infra/main.tf", "# infra\n");
        write(&viz, ".task/005.md", "# task\n");
        write(&viz, "README.md", "# readme\n");
        write(&viz, "LICENSE", "license\n");
        write(&viz, "public/graph.json", "{}\n");
        write(&viz, "public/graph-sim.bin", "bin\n");
        write(&viz, "public/sitemap.xml", "<urlset/>\n");
        commit(&viz, "vizualisation");
        write(&dir.join("ui-lib"), "src/lib.rs", "// ui\n");
        commit(&dir.join("ui-lib"), "ui-lib");
        Self { dir }
    }

    fn repo(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// The script's output and exit success.
    fn run(&self) -> (bool, String) {
        let output = Command::new("bash")
            .arg(self.repo("vizualisation").join("scripts/image-hash.sh"))
            .arg(self.repo("ui-lib"))
            .output()
            .expect("run the script");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), text.trim().to_string())
    }

    fn hash(&self) -> String {
        let (ok, out) = self.run();
        assert!(ok, "the script failed: {out}");
        out
    }

    /// Change one file of a repository and commit it.
    fn change(&self, repo: &str, file: &str, text: &str) {
        write(&self.repo(repo), file, text);
        commit(&self.repo(repo), "change");
    }
}

#[test]
fn the_hash_is_h_and_sixteen_hex() {
    let w = Workspace::new("shape");
    let hash = w.hash();
    let hex = hash.strip_prefix("h-").expect("an h- prefix");
    assert_eq!(hex.len(), 16, "{hash}");
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "{hash}");
}

#[test]
fn the_same_inputs_give_the_same_hash() {
    let w = Workspace::new("same");
    assert_eq!(w.hash(), w.hash());
}

#[test]
fn each_input_changes_the_hash() {
    let cases: [(&str, &str, &str); 12] = [
        ("vizualisation", "app/src/lib.rs", "pub fn b() {}\n"),
        ("vizualisation", "engine/src/lib.rs", "// engine 2\n"),
        ("vizualisation", "layout/src/lib.rs", "// layout 2\n"),
        ("vizualisation", "formats/src/lib.rs", "// formats 2\n"),
        // app/build.rs's placeholder check (task 012), copied by the Dockerfile.
        (
            "vizualisation",
            "i18n-check/src/lib.rs",
            "// i18n-check 2\n",
        ),
        ("vizualisation", "Dockerfile", "FROM scratch\nRUN true\n"),
        ("vizualisation", "scripts/smoke.sh", "echo changed\n"),
        ("vizualisation", "Cargo.lock", "# lock 2\n"),
        (
            "vizualisation",
            "Cargo.toml",
            "[workspace]\nresolver = \"3\"\n",
        ),
        ("ui-lib", "src/lib.rs", "// ui 2\n"),
        // The data the image carries (Dockerfile, `./data/`).
        ("vizualisation", "public/graph.json", "[]\n"),
        ("vizualisation", "public/graph-sim.bin", "bin 2\n"),
    ];
    for (i, (repo, file, text)) in cases.iter().enumerate() {
        let w = Workspace::new(&format!("input-{i}"));
        let before = w.hash();
        w.change(repo, file, text);
        assert_ne!(before, w.hash(), "{repo}/{file} must change the hash");
    }
}

#[test]
fn files_outside_the_inputs_do_not() {
    for (i, (file, text)) in [
        ("infra/main.tf", "# infra 2\n"),
        (".task/005.md", "# task 2\n"),
        ("README.md", "# readme 2\n"),
        ("LICENSE", "license 2\n"),
        ("public/sitemap.xml", "<urlset></urlset>\n"),
    ]
    .iter()
    .enumerate()
    {
        let w = Workspace::new(&format!("outside-{i}"));
        let before = w.hash();
        w.change("vizualisation", file, text);
        assert_eq!(before, w.hash(), "{file} is not an input");
    }
}

#[test]
fn an_untracked_file_is_not_an_input() {
    let w = Workspace::new("untracked");
    let before = w.hash();
    write(
        &w.repo("vizualisation"),
        "app/src/scratch.rs",
        "// not committed\n",
    );
    assert_eq!(before, w.hash());
}

#[test]
fn a_missing_sibling_fails_with_a_message() {
    let w = Workspace::new("missing");
    fs::remove_dir_all(w.repo("ui-lib").join(".git")).expect("remove .git");
    let (ok, out) = w.run();
    assert!(!ok, "a sibling that is no git checkout must fail: {out}");
    assert!(
        out.contains("ui-lib"),
        "the message names the sibling: {out}"
    );
}

#[test]
fn missing_arguments_fail_with_a_usage_line() {
    let w = Workspace::new("usage");
    let output = Command::new("bash")
        .arg(w.repo("vizualisation").join("scripts/image-hash.sh"))
        .output()
        .expect("run the script");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage"));
}
