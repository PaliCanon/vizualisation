#!/usr/bin/env python3
"""Checks the rules the CI design names, on the workflows and composite actions.

From search-front/scripts/check_workflows.py (task 007 there). What differs:
one sibling (ui-lib, no api) in the messages; a job that uses a local
composite action whose steps run cargo needs the siblings too (the React
site's job built the engine through such an action until task 008 removed
both; the rule stays for the next one); and every test file of the
repository has its step in ci.yml (task 005, S5).

No actionlint on the laptop and none in the job: these rules are the ones the
deploy design depends on, checked as YAML.

  python3 scripts/check_workflows.py

Rules:
- every file parses as YAML;
- a workflow declares `permissions` (and a job may), read-only except
  `packages: write` (the push to GHCR); `id-token: write` is not allowed;
- no `${{ secrets.* }}` inside a `run:` line (a secret goes through `env:` or
  `with:`, never into a shell command that a log or `set -x` could show);
- every `uses:` names a version (`@...`), local actions apart;
- every `actions/checkout` sets `persist-credentials: false` (the GitHub App
  token must not stay in `.git/config`);
- every job that runs `cargo` has the `siblings` action, `cargo fmt` included
  (rustfmt follows `app/build.rs`'s `#[path]` into ui-lib, F4 of 007), and so
  does a job using a local composite action that runs cargo;
- every test file (`scripts/*_test.py`, `scripts/*.test.sh`) is named, as a
  path or a glob, by a `run:` of ci.yml (a test no job runs guards nothing);
- `node --test` takes files or quoted globs, never a directory (node 22 runs
  a directory as a module and fails with "Cannot find module", F3 of 007);
- no `docker/build-push-action` takes `secrets:`, `ssh:` or `build-args` (the
  build needs no credential, ruling R1, and the release is not in the image,
  task 016);
- deploy.yml (a tag v* builds the image and pushes it to GHCR; prod, on the
  lsh-tech server, pulls it by digest): the concurrency group deploy-prod,
  which never cancels a build in progress, and
  (a) `no-gcp`: no `gcloud`, `google-github-actions`, `docker.pkg.dev` or
      `run.app` anywhere in the file;
  (b) `job-permissions`: the job that builds declares `permissions` with
      `packages: write` and `contents: read`, and no `id-token`;
  (c) `login`: a `docker/login-action` on registry `ghcr.io`, with username
      `${{ github.actor }}` and password `${{ secrets.GITHUB_TOKEN }}`, comes
      before the build; no other registry is logged into (no other
      `docker/login-action`, no `docker login` in a `run:`);
  (d) `image-refs`: the image is `ghcr.io/palicanon/...`, its hash tag comes
      from `scripts/image-hash.sh`, and the build's `tags:` hold the hash tag
      (`outputs.image_hash`) AND the release tag (`github.ref_name`);
  (e) `labels`: the build's `labels:` carry `org.opencontainers.image.source=`;
  (f) `existence`: the check that writes `exists=` runs
      `docker buildx imagetools inspect`, `docker/setup-buildx-action` runs
      before it with no `if:`, and the build is skipped when the image
      exists (`if:` on `steps.<check>.outputs.exists != 'true'`);
  (g) `summary`: a step after the build writes to `$GITHUB_STEP_SUMMARY` the
      image's digest, pinned to the image with `@` (what the stack is updated
      with by hand).
Exit 0 clean, 1 with one line per violation.
"""

import fnmatch
import pathlib
import re
import sys

import yaml


def steps_of(doc):
    """Every step of a workflow (all jobs) or of a composite action."""
    if "jobs" in doc:
        for job in doc["jobs"].values():
            yield from job.get("steps", [])
    else:
        yield from doc.get("runs", {}).get("steps", [])


def too_wide(perms):
    """The scopes of `perms` that are more than read: only `packages: write` may be."""
    return [
        scope for scope, level in perms.items()
        if level not in ("read", "none") and not (scope == "packages" and level == "write")
    ]


def check_permissions(name, doc):
    if "jobs" not in doc:
        return []
    perms = doc.get("permissions")
    if not isinstance(perms, dict):
        return [f"{name}: permissions: a workflow must declare a minimal `permissions`"]
    found = [
        f"{name}: permissions: `{scope}` is more than read (only packages: write is allowed)"
        for scope in too_wide(perms)
    ]
    for job_name, job in doc["jobs"].items():
        if "permissions" not in job:
            continue
        job_perms = job["permissions"]
        if not isinstance(job_perms, dict):
            found.append(f"{name}: permissions: job `{job_name}` must list its scopes, read-only except packages: write")
            continue
        found += [
            f"{name}: permissions: job `{job_name}`: `{scope}` is more than read (only packages: write is allowed)"
            for scope in too_wide(job_perms)
        ]
    return found


def check_node_test(name, run):
    """`node --test DIR` runs the directory as a module on node 22 (task 007
    F3): the arguments must be files or quoted globs (node expands them)."""
    found = []
    for line in run.splitlines():
        match = re.search(r"\bnode\s+--test\s+(\S+)", line)
        if match and not (match.group(1)[0] in "'\"" or match.group(1).endswith((".mjs", ".js", ".cjs"))):
            found.append(f"{name}: node --test: `{match.group(1)}` is a directory or an unquoted glob; node 22 runs it as a module (use a quoted glob or a file)")
    return found


def check_step(name, step):
    found = []
    run = step.get("run", "")
    if "secrets." in run:
        found.append(f"{name}: secret in a run line: {run.strip().splitlines()[0][:60]}")
    found.extend(check_node_test(name, run))
    uses = step.get("uses", "")
    if uses and not uses.startswith("./") and "@" not in uses:
        found.append(f"{name}: `{uses}` names no version")
    with_ = step.get("with", {}) or {}
    if uses.startswith("actions/checkout@") or uses == "actions/checkout":
        if with_.get("persist-credentials") is not False:
            found.append(f"{name}: persist-credentials: an actions/checkout must set it to false")
    if uses.startswith("docker/build-push-action"):
        for key in ("secrets", "ssh"):
            if key in with_:
                found.append(f"{name}: build-push: `{key}:` is refused, the image build takes no credential")
    return found


def check_build_args(name, doc):
    """R1: the image build takes no build args.

    Design sentence (016, 060): "No build args: the release is not in the
    image, so its hash does not change with the ref."
    """
    return [
        f"{name}: build-args: the image takes no build args (the release is not in the image: its hash must not change with the ref)"
        for step in steps_of(doc)
        if step.get("uses", "").startswith("docker/build-push-action")
        and "build-args" in (step.get("with") or {})
    ]


def logical_lines(run):
    """The command lines of a shell script: continuations joined, comments dropped."""
    joined = re.sub(r"\\\n\s*", " ", run)
    return [
        line.strip()
        for line in joined.splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]


def local_actions_running_cargo(actions):
    """The names (`build`) of the local composite actions whose steps run cargo."""
    return {
        label.split("/")[0]
        for label, doc in actions.items()
        if any(re.search(r"(^|[\s&;|(])cargo\s", step.get("run", ""), re.M) for step in steps_of(doc or {}))
    }


def check_siblings(name, doc, cargo_actions=frozenset()):
    """Every job that runs cargo has the `siblings` action (search-front 007
    F4): the workspace reaches `../ui-lib`, and rustfmt follows
    `app/build.rs`'s `#[path]` into ui-lib, so even `cargo fmt` needs it. A
    job that uses a local action running cargo (`cargo_actions`) runs cargo."""
    found = []
    for job_name, job in (doc.get("jobs") or {}).items():
        steps = job.get("steps", [])
        runs_cargo = any(re.search(r"(^|[\s&;|(])cargo\s", step.get("run", ""), re.M) for step in steps) or any(
            str(step.get("uses", "")).startswith("./") and str(step.get("uses", "")).rstrip("/").rsplit("/", 1)[-1] in cargo_actions
            for step in steps
        )
        has_siblings = any(str(step.get("uses", "")).endswith("/siblings") for step in steps)
        if runs_cargo and not has_siblings:
            found.append(f"{name}: job `{job_name}` runs cargo without the siblings action (ui-lib beside the checkout)")
    return found


# The test files of the repository, each run by a step of ci.yml.
TEST_GLOBS = ("scripts/*_test.py", "scripts/*.test.sh")


def test_files(root):
    """The repository's test files, as paths relative to `root`."""
    return sorted({str(p.relative_to(root)) for pattern in TEST_GLOBS for p in root.glob(pattern)})


def check_test_steps(name, doc, files):
    """Every file of `files` is named by a `run:` of the workflow `doc`, as a
    path or as a (quoted) glob; a comment does not count."""
    words = [
        word.strip("'\"")
        for step in steps_of(doc or {})
        for line in logical_lines(step.get("run", ""))
        for word in line.split("#", 1)[0].split()
    ]
    return [
        f"{name}: {path} is run by no step (every test file has its CI step)"
        for path in files
        if not any(fnmatch.fnmatchcase(path, word) for word in words)
    ]


GCP_NEEDLES = ("gcloud", "google-github-actions", "docker.pkg.dev", "run.app")


def strings_of(node):
    """Every string of a parsed YAML tree, keys and values (comments are not parsed, so not in it)."""
    if isinstance(node, str):
        yield node
    elif isinstance(node, dict):
        for key, value in node.items():
            yield from strings_of(key)
            yield from strings_of(value)
    elif isinstance(node, (list, tuple)):
        for item in node:
            yield from strings_of(item)


def check_no_gcp(name, doc):
    """(a) Prod is the lsh-tech server and the registry is GHCR: no Google Cloud."""
    return [
        f"{name}: no-gcp: `{needle}` appears in the file; deploy.yml only builds the image and pushes it to GHCR"
        for needle in GCP_NEEDLES
        if any(needle in text for text in strings_of(doc))
    ]


def is_build(step):
    return str(step.get("uses", "")).startswith("docker/build-push-action")


def first_build(steps):
    return next((at for at, step in enumerate(steps) if is_build(step)), None)


def with_of(step):
    return step.get("with") or {}


def check_job_permissions(name, job_name, job):
    """(b) The job that pushes the image can write packages, read the code, and mint no OIDC token."""
    perms = job.get("permissions")
    if not isinstance(perms, dict):
        return [f"{name}: job-permissions: job `{job_name}` builds the image and needs `permissions` with `packages: write` and `contents: read`"]
    found = []
    if perms.get("packages") != "write":
        found.append(f"{name}: job-permissions: job `{job_name}` needs `packages: write` (the push to GHCR)")
    if perms.get("contents") != "read":
        found.append(f"{name}: job-permissions: job `{job_name}` needs `contents: read`")
    if "id-token" in perms:
        found.append(f"{name}: job-permissions: job `{job_name}` must not have `id-token` (no OIDC: nothing in this deploy trusts a token)")
    return found


GHCR_LOGIN = {
    "username": "${{ github.actor }}",
    "password": "${{ secrets.GITHUB_TOKEN }}",
}


def check_login(name, steps, build):
    """(c) One login, on ghcr.io, with the actor and the GITHUB_TOKEN, before the build."""
    found = []
    for step in steps:
        if re.search(r"\bdocker\s+login\b", "\n".join(logical_lines(step.get("run", "")))):
            found.append(f"{name}: login: `docker login` in a run line; use docker/login-action with the token in `with:`")
    logins = [(at, step) for at, step in enumerate(steps) if str(step.get("uses", "")).startswith("docker/login-action")]
    for _, step in logins:
        registry = str(with_of(step).get("registry", "")).strip()
        if registry != "ghcr.io":
            found.append(f"{name}: login: a docker/login-action on registry `{registry or 'docker.io (none given)'}`: only ghcr.io is logged into")
    ghcr = [(at, step) for at, step in logins if str(with_of(step).get("registry", "")).strip() == "ghcr.io"]
    if not ghcr:
        found.append(f"{name}: login: no docker/login-action with `registry: ghcr.io` (the push needs it)")
        return found
    at, step = ghcr[0]
    for key, expected in GHCR_LOGIN.items():
        if str(with_of(step).get(key, "")).strip() != expected:
            found.append(f"{name}: login: the ghcr.io login's {key} must be `{expected}`")
    if at > build:
        found.append(f"{name}: login: the ghcr.io login must come before the build")
    return found


def check_image_refs(name, steps, build):
    """(d) The image is ghcr.io/palicanon/..., from the hash script, and tagged with the hash and the release."""
    found = []
    runs = "\n".join("\n".join(logical_lines(step.get("run", ""))) for step in steps)
    images = re.findall(r"^IMAGE=(\S+)", runs, re.M)
    if not images or not all(image.startswith("ghcr.io/palicanon/") for image in images):
        found.append(f"{name}: image-refs: the image must be `IMAGE=ghcr.io/palicanon/...`")
    if "scripts/image-hash.sh" not in runs:
        found.append(f"{name}: image-refs: the hash tag must come from scripts/image-hash.sh")
    tags = [line.strip() for line in str(with_of(steps[build]).get("tags", "")).splitlines()]
    if not any("outputs.image_hash" in line for line in tags):
        found.append(f"{name}: image-refs: the build's tags must hold the hash tag (steps.<id>.outputs.image_hash)")
    if not any(re.search(r":\$\{\{\s*github\.ref_name\s*\}\}$", line) for line in tags):
        found.append(f"{name}: image-refs: the build's tags must hold the release tag (`<image>:${{{{ github.ref_name }}}}`)")
    return found


def check_labels(name, steps, build):
    """(e) The source label links the package to the repository."""
    if "org.opencontainers.image.source=" in str(with_of(steps[build]).get("labels", "")):
        return []
    return [f"{name}: labels: the build's `labels:` must carry org.opencontainers.image.source= (it links the GHCR package to the repository)"]


def check_existence(name, steps, build):
    """(f) `imagetools inspect` tells whether the image exists, buildx is there before, and the build is skipped when it does."""
    found = []
    check_at = next((at for at, step in enumerate(steps) if "exists=" in step.get("run", "")), None)
    if check_at is None:
        return [f"{name}: existence: no step writes `exists=` (the check of an image already built)"]
    check = steps[check_at]
    if not any(re.search(r"\bdocker\s+buildx\s+imagetools\s+inspect\b", line) for line in logical_lines(check["run"])):
        found.append(f"{name}: existence: the check must run `docker buildx imagetools inspect <image>:<hash tag>`")
    expected = f"steps.{check.get('id', '<id>')}.outputs.exists != 'true'"
    if " ".join(str(steps[build].get("if", "")).split()) != expected:
        found.append(f"{name}: existence: the build must be skipped when the image exists (`if: {expected}`)")
    buildx = next((at for at, step in enumerate(steps) if str(step.get("uses", "")).startswith("docker/setup-buildx-action")), None)
    if buildx is None:
        found.append(f"{name}: existence: `docker buildx imagetools` needs docker/setup-buildx-action")
    else:
        if buildx > check_at:
            found.append(f"{name}: existence: docker/setup-buildx-action must run before the existence check")
        if "if" in steps[buildx]:
            found.append(f"{name}: existence: docker/setup-buildx-action runs unconditionally (no `if:`): the check needs it whether or not the build runs")
    return found


PINNED_DIGEST = re.compile(r"@(\$\{?\w*digest\w*\}?|sha256)", re.I)


def check_summary(name, steps, build):
    """(g) A step after the build writes the image's digest, pinned with `@`, to the job summary."""
    writers = [at for at, step in enumerate(steps) if "GITHUB_STEP_SUMMARY" in step.get("run", "")]
    if not writers:
        return [f"{name}: summary: no step writes the digest to $GITHUB_STEP_SUMMARY (what the stack is updated with by hand)"]
    found = []
    after = [at for at in writers if at > build]
    if not after:
        return [f"{name}: summary: the step writing $GITHUB_STEP_SUMMARY must come after the build"]
    run = "\n".join(steps[at]["run"] for at in after)
    if "digest" not in run.lower() or not PINNED_DIGEST.search(run):
        found.append(f"{name}: summary: the summary must give the image's digest pinned with `@` (<image>@<digest>)")
    return found


def check_image_job(name, job_name, job):
    steps = job.get("steps", [])
    build = first_build(steps)
    if build is None:
        return []
    found = check_job_permissions(name, job_name, job)
    for rule in (check_login, check_image_refs, check_labels, check_existence, check_summary):
        found += rule(name, steps, build)
    return found


def check_deploy(name, doc):
    found = []
    group = str(doc.get("concurrency", {}).get("group", ""))
    if group != "deploy-prod":
        found.append(f"{name}: concurrency: the group must be deploy-prod (one deploy at a time)")
    if doc.get("concurrency", {}).get("cancel-in-progress") is not False:
        found.append(f"{name}: concurrency: cancel-in-progress must be false (never cancel a build in progress)")
    found += check_no_gcp(name, doc)
    found += check_build_args(name, doc)
    jobs = doc.get("jobs") or {}
    if not any(first_build(job.get("steps", [])) is not None for job in jobs.values()):
        found.append(f"{name}: image: no job builds the image (docker/build-push-action)")
    for job_name, job in jobs.items():
        found += check_image_job(name, job_name, job)
    return found


def check(workflows, actions):
    """Violations of `workflows` and `actions` (name → parsed YAML)."""
    found = []
    for name, doc in {**workflows, **actions}.items():
        found += check_permissions(name, doc)
        for step in steps_of(doc):
            found += check_step(name, step)
    cargo_actions = local_actions_running_cargo(actions)
    for name, doc in workflows.items():
        found += check_siblings(name, doc, cargo_actions)
        if name == "deploy.yml":
            found += check_deploy(name, doc)
    return found


def load(path, label, into, problems):
    """Parse one YAML file into `into[label]`, or record why it does not."""
    try:
        into[label] = yaml.safe_load(path.read_text())
    except yaml.YAMLError as err:
        problems.append(f"{label}: not valid YAML: {str(err).splitlines()[0]}")


def check_repository(root):
    """Load the repository's workflows and composite actions, then `check`."""
    workflows, actions, problems = {}, {}, []
    for path in sorted((root / ".github" / "workflows").glob("*.yml")):
        load(path, path.name, workflows, problems)
    for path in sorted((root / ".github" / "actions").glob("*/action.yml")):
        load(path, f"{path.parent.name}/action.yml", actions, problems)
    if not workflows and not problems:
        return ["no workflow found under .github/workflows"]
    tests = check_test_steps("ci.yml", workflows.get("ci.yml"), test_files(root))
    return problems + check(workflows, actions) + tests


if __name__ == "__main__":
    problems = check_repository(pathlib.Path(__file__).resolve().parent.parent)
    for line in problems:
        print(line, file=sys.stderr)
    print(f"check_workflows: {len(problems)} violation(s)")
    sys.exit(1 if problems else 0)
