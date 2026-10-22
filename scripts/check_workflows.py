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
- a workflow declares `permissions`, read-only except `id-token: write`;
- no `${{ secrets.* }}` inside a `run:` line (a secret goes through `env:` or
  `with:`, never into a shell command that a log or `set -x` could show);
- every `uses:` names a version (`@...`), local actions apart;
- every `actions/checkout` sets `persist-credentials: false` (the GitHub App
  token must not stay in `.git/config`);
- every job that runs `cargo` has the `siblings` action, `cargo fmt` included
  (rustfmt follows `app/build.rs`'s `#[path]` into ui-lib, F4 of 007), and so
  does a job using a local composite action that runs cargo;
- every test file (`scripts/*_test.py`, `scripts/*.test.sh`,
  `infra/scripts/*.test.sh`, `infra/worker/*.test.mjs`) is named, as a path or
  a glob, by a `run:` of ci.yml (a test no job runs guards nothing);
- `node --test` takes files or quoted globs, never a directory (node 22 runs
  a directory as a module and fails with "Cannot find module", F3 of 007);
- no `docker/build-push-action` takes `secrets:` or `ssh:` (the build needs
  no credential, ruling R1);
- deploy.yml: a concurrency group per environment that never cancels a
  rollout in progress; `rollout-check.sh` called by its real interface (the
  revision URL with the header file before the traffic update, `--via-worker`
  on the public host after it, `--refused` on the bare Cloud Run URL; the
  last two never given the secret; exit 3, a Cloudflare challenge, a
  `::warning::` and not a failure); no inline `curl`; the image build given
  no build args (the release is not in the image, task 016); the
  "already running" skip compares the image of the revision that SERVES the
  traffic (read by `scripts/serving-revision.sh`, which falls back to the
  latest ready revision when the 100% entry follows the latest; an empty
  result fails the job with an `::error::`), never `spec.template`, which
  already holds a half-finished rollout's new image. The public-host step
  rolls the traffic back (`update-traffic --to-revisions`, in a `rollback`
  function whose failure is reported and never masks the check's exit code)
  only in the arm of exit 1 (a refusal or a failed content check) of
  `--via-worker`; never on exit 3 (a Cloudflare challenge), nor on exit 4
  (the Worker answered 500: an `::error::` naming `origin-secret push`), nor
  when the bare URL fails `--refused` (both revisions boot from the same env:
  a rollback restores nothing, so the job fails with an `::error::` naming
  the service's `ORIGIN_SECRET` env and the traffic is left as is). The arms
  are read as arms (`shell_contexts`), not as text offsets. The `rollback`
  function observes its outcome: after the update it reads `status.traffic`
  back (`describe | scripts/serving-revision.sh`) and, when the previous
  revision does not serve, ends in `::error::rollback did not restore <prev>`
  and `exit 5`; otherwise the caller's arm exits with the check's code.
- the release wiring (task 016, twin of learn-pali 060): R1 no
  `docker/build-push-action` has `build-args`; R2 `--set-env-vars` appears in
  no `run:` line (it would wipe every other variable of the revision); R3
  every `gcloud run deploy` carries `--update-env-vars
  "$(scripts/release-values.sh)"`, and one exists; R4 the "already running"
  step compares the release with `scripts/release-values.sh same`; R5 the step
  calling the direct `rollout-check.sh` sets `ROLLOUT_EXPECT_ENV` and
  `ROLLOUT_EXPECT_SHA` from `release-values.sh env` / `sha` before the call
  and has an arm for exit 8 that ends in an `::error::` and an `exit`.
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


def check_permissions(name, doc):
    if "jobs" not in doc:
        return []
    perms = doc.get("permissions")
    if not isinstance(perms, dict):
        return [f"{name}: permissions: a workflow must declare a minimal `permissions`"]
    bad = [
        scope for scope, level in perms.items()
        if level not in ("read", "none") and not (scope == "id-token" and level == "write")
    ]
    return [f"{name}: permissions: `{scope}` is more than read (only id-token: write is allowed)" for scope in bad]


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


def run_lines(doc):
    """Every non-comment line of every `run:` of the workflow, in order."""
    lines = []
    for step in steps_of(doc):
        for line in step.get("run", "").splitlines():
            if not line.strip().startswith("#"):
                lines.append(line)
    return lines


def check_build_args(name, doc):
    """R1: the image build takes no build args.

    Design sentence (016, 060): "No build args: the release is not in the
    image, so its hash does not change with the ref." The release is written
    onto the revision by the rollout step.
    """
    return [
        f"{name}: build-args: the image takes no build args (the release is the revision's env, not the image's)"
        for step in steps_of(doc)
        if step.get("uses", "").startswith("docker/build-push-action")
        and "build-args" in (step.get("with") or {})
    ]


def check_no_set_env_vars(name, doc):
    """R2: `--set-env-vars` is never used.

    It replaces every variable of the revision and would wipe `RUST_LOG`,
    `VIZ_GATE` and the origin secret reference; the deploy only updates the
    release's two (`--update-env-vars`). Comment lines are not uses.
    """
    return [
        f"{name}: --set-env-vars: never used, it wipes every other variable of the revision (use --update-env-vars): {line.strip()[:60]}"
        for line in run_lines(doc)
        if "--set-env-vars" in line
    ]


RELEASE_FLAG = '--update-env-vars "$(scripts/release-values.sh)"'


def check_deploy_writes_release(name, doc):
    """R3: every `gcloud run deploy` writes the release, and one exists.

    Design sentence: "gcloud run deploy ... --update-env-vars
    "$(scripts/release-values.sh)" --no-traffic" (the values come from the
    script, never a literal).
    """
    deploys = [
        line
        for step in steps_of(doc)
        for line in logical_lines(step.get("run", ""))
        if re.search(r"\bgcloud\s+run\s+deploy\b", line)
    ]
    if not deploys:
        return [f"{name}: --update-env-vars: no step runs `gcloud run deploy`, so nothing writes the release"]
    return [
        f"{name}: --update-env-vars: a `gcloud run deploy` must carry {RELEASE_FLAG} (the release from the script): {line[:60]}"
        for line in deploys
        if RELEASE_FLAG not in line
    ]


def check_skip_compares_release(name, doc):
    """R4: the "already running" step compares the release, not only the image.

    Design sentence: "Already running means the serving revision has this
    image AND this release" (`scripts/release-values.sh same`).
    """
    skip = next((s for s in steps_of(doc) if "running=" in s.get("run", "")), None)
    if skip is None or any("release-values.sh same" in l for l in logical_lines(skip["run"])):
        return []
    return [f"{name}: skip: the step writing `running=` must compare the release with `scripts/release-values.sh same` (the image no longer changes with the ref)"]


def check_gate_release(name, doc):
    """R5: the direct gate is given the release and its exit 8 is reported.

    Design sentence: the gate asserts the page serves the deployed release
    (`ROLLOUT_EXPECT_ENV`, `ROLLOUT_EXPECT_SHA`, exit 8 = another release):
    the step sets both from `release-values.sh env` / `sha` before the call
    and has an arm for exit 8 that ends in an `::error::` and an `exit`.
    """
    step = next(
        (
            s for s in steps_of(doc)
            if any(
                "rollout-check.sh" in l and "--via-worker" not in l and "--refused" not in l
                for l in logical_lines(s.get("run", ""))
            )
        ),
        None,
    )
    if step is None:
        return []
    lines = shell_contexts(step["run"])
    call = next(at for at, (l, _) in enumerate(lines) if "rollout-check.sh" in l)
    found = []
    for var, mode in (("ROLLOUT_EXPECT_ENV", "env"), ("ROLLOUT_EXPECT_SHA", "sha")):
        pattern = rf'^(export\s+)?{var}="?\$\(scripts/release-values\.sh {mode}\)"?$'
        if not any(re.match(pattern, l) for l, _ in lines[:call]):
            found.append(f'{name}: release: {var}="$(scripts/release-values.sh {mode})" must be set before the direct rollout-check.sh call')
    arm = [(l, c) for l, c in lines if any(_exit_code_test("8").search(x) for x in c)]
    if not arm:
        found.append(f"{name}: release: the direct rollout-check.sh call needs an arm for exit 8 (the page serves another release)")
        return found
    if not any("::error::" in l for l, _ in arm):
        found.append(f"{name}: release: the exit 8 arm must report an ::error:: (another release than the deployed one)")
    if not any(l.startswith("exit") for l, _ in arm):
        found.append(f"{name}: release: the exit 8 arm must end in an exit")
    return found


def check_rollout_calls(name, doc):
    """`scripts/rollout-check.sh [--via-worker | --refused] URL [HEADER_FILE]`."""
    found = []
    lines = run_lines(doc)
    calls = {"direct": [], "--via-worker": [], "--refused": []}
    for at, line in enumerate(lines):
        if "rollout-check.sh" not in line:
            continue
        mode = next((m for m in ("--via-worker", "--refused") if m in line), "direct")
        calls[mode].append(at)
        if mode != "direct" and re.search(r"HEADER_FILE|ORIGIN_SECRET|origin-header", line):
            found.append(f"{name}: rollout-check.sh {mode} must not be given the secret: {line.strip()[:60]}")
    traffic = next((at for at, line in enumerate(lines) if "update-traffic" in line), -1)
    if not calls["direct"]:
        found.append(f"{name}: no step runs scripts/rollout-check.sh URL HEADER_FILE on the tagged revision")
    if traffic < 0:
        found.append(f"{name}: no step moves the traffic (update-traffic)")
    if calls["direct"] and 0 <= traffic < calls["direct"][0]:
        found.append(f"{name}: traffic is moved before rollout-check.sh runs")
    for mode in ("--via-worker", "--refused"):
        if not calls[mode]:
            found.append(f"{name}: no step runs scripts/rollout-check.sh {mode} URL")
    if calls["--via-worker"] and 0 <= traffic and calls["--via-worker"][0] < traffic:
        found.append(f"{name}: rollout-check.sh --via-worker runs before the traffic moves")
    if calls["--via-worker"]:
        step = next(
            s for s in steps_of(doc) if "--via-worker" in s.get("run", "")
        )
        if "::warning::" not in step["run"] or not re.search(r"\b3\b", step["run"]):
            found.append(f"{name}: rollout-check.sh --via-worker: exit 3 (a Cloudflare challenge) must end in a ::warning::, not a failure")
    if any(re.search(r"\bcurl\b", line) for line in lines):
        found.append(f"{name}: inline curl in a run line: the checks go through scripts/rollout-check.sh")
    return found


def check_serving_skip(name, doc):
    """The skip compares what serves, and the serving revision is recorded (D7)."""
    steps = list(steps_of(doc))
    skip = next((s for s in steps if "running=" in s.get("run", "")), None)
    if skip is None:
        return [f"{name}: no step writes `running=` (the skip of an image already served)"]
    run = skip["run"]
    found = []
    if "spec.template" in run:
        found.append(f"{name}: skip: compare the image of the serving revision (the one at 100% of the traffic), not spec.template")
    elif "revisions describe" not in run:
        found.append(f"{name}: skip: read the image of the serving revision (gcloud run revisions describe)")
    call = next((l for l in logical_lines(run) if "serving-revision.sh" in l), None)
    if call is None:
        found.append(
            f"{name}: skip: read the serving revision with scripts/serving-revision.sh "
            "(the 100% entry, or the latest ready revision when it follows the latest)"
        )
    else:
        if "::error::" not in call:
            found.append(f"{name}: skip: an empty serving revision must fail the job with an ::error:: (`|| {{ echo \"::error::...\"; exit 1; }}`)")
        if "pipefail" not in run:
            found.append(f"{name}: skip: `set -o pipefail` is needed, or the pipe to serving-revision.sh hides its failure")
    if "serving=" not in run:
        found.append(f"{name}: skip: record `serving=<revision>` in the step output, before the traffic moves")
    return found


def logical_lines(run):
    """The command lines of a shell script: continuations joined, comments dropped."""
    joined = re.sub(r"\\\n\s*", " ", run)
    return [
        line.strip()
        for line in joined.splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]


_IF = re.compile(r"^(if|elif)\s+(.*?);\s*then\b\s*(.*)$")
_FN = re.compile(r"^([A-Za-z_]\w*)\(\)\s*\{$")


class Arm(str):
    """The test of one `if`/`elif`/`else` arm; two arms are never equal, even with the same text."""

    def __eq__(self, other):
        return self is other

    def __ne__(self, other):
        return self is not other

    __hash__ = object.__hash__


def shell_contexts(run):
    """`(line, conditions)` for every command line of a shell script.

    `conditions` are the tests of the `if`/`elif` arms (and `else`) enclosing
    the line, outermost first, so a rule can say which arm a command sits in
    instead of where it sits in the text. A function body is marked
    `fn <name>`. One-line `if ...; then ...; fi` is handled like the long form.
    """
    stack, found = [], []

    def emit(command, closes):
        command = command.strip().rstrip(";").strip()
        if command:
            found.append((command, tuple(stack)))
        if closes:
            stack.pop()

    for line in logical_lines(run):
        opened = _IF.match(line)
        if opened:
            keyword, condition, rest = opened.groups()
            if keyword == "if":
                stack.append(Arm(condition))
            else:
                stack[-1] = Arm(condition)
            closes = rest.endswith("; fi") or rest == "fi"
            emit(re.sub(r";?\s*fi$", "", rest), closes)
        elif line == "else":
            stack[-1] = Arm("else")
        elif line == "fi" or line == "}":
            stack.pop()
        elif line == "done":
            stack.pop()
        elif _FN.match(line):
            stack.append(Arm("fn " + _FN.match(line).group(1)))
        elif re.match(r"^(for|while|until)\b.*;\s*do$", line):
            stack.append(Arm(line))
        else:
            emit(line, False)
    return found


def _exit_code_test(code):
    """A regex for a test of an `RC`-like variable against `code`."""
    return re.compile(r"\$\{?(?:rc|RC)\}?\"?\s*(?:==?|-eq)\s*\"?" + code + r"\b")


REFUSAL, CHALLENGE, WORKER_500 = (_exit_code_test(code) for code in "134")


def rollback_sites(lines):
    """`(at, line, conditions)` of every rollback, `lines` being `shell_contexts`.

    A `--to-revisions` line inside a function counts where the function is
    called: its sites are the calls, with the callers' conditions. A function
    nobody calls keeps its own (`fn <name>`), which no rule accepts.
    """
    sites = []
    for at, (line, conditions) in enumerate(lines):
        if "--to-revisions" not in line:
            continue
        function = next((str(c)[3:] for c in conditions if str(c).startswith("fn ")), None)
        calls = [
            (call, line, c)
            for call, (text, c) in enumerate(lines)
            if function and re.search(rf"(?:^|[;&|]\s*){re.escape(function)}(?:\s|;|$)", text)
        ]
        sites += calls or [(at, line, conditions)]
    return sites


_RC_EXIT = re.compile(r"^exit\s+\"?\$\{?(?:rc|RC)\}?\"?$")
_GUARDED = re.compile(r"\|\|\s*echo\s+\"::error::rollback\b")


def check_rollback_failure(name, lines, sites):
    """The rollback is guarded and its arm still exits with the check's code.

    Under `set -e` a failing `update-traffic` would end the step with
    gcloud's exit code and hide the check's: the failure is reported with
    `|| echo "::error::rollback ... failed"` and the arm then `exit "$RC"`.
    """
    found = []
    exits = [c for line, c in lines if _RC_EXIT.match(line)]
    for _, line, conditions in sites:
        if not _GUARDED.search(line):
            found.append(
                f"{name}: rollback: a failing update-traffic would mask the check's exit code "
                '(set -e): end it with `|| echo "::error::rollback to $PREVIOUS failed"`'
            )
        if not any(conditions[: len(c)] == c and c for c in exits):
            found.append(
                f"{name}: rollback: the arm must go on to `exit \"$RC\"`, the check's exit code"
            )
    return found


_EXIT_5 = re.compile(r"^exit\s+5$")


def check_rollback_outcome(name, lines):
    """The rollback is observed, and a rollback that did not restore exits 5.

    After `update-traffic --to-revisions`, inside the `rollback` function, the
    traffic is read back (`scripts/serving-revision.sh` on `describe`, that is
    `status.traffic`); when the previous revision does not serve, an
    `::error::rollback did not restore <prev>` and `exit 5`. Otherwise the
    caller's arm ends with the check's own exit code.
    """
    body = [(line, c) for line, c in lines if any(str(x) == "fn rollback" for x in c)]
    update = next((at for at, (line, _) in enumerate(body) if "--to-revisions" in line), None)
    if update is None:
        return []
    found = []
    if not any("serving-revision.sh" in line for line, _ in body[update + 1 :]):
        found.append(
            f"{name}: rollback: after update-traffic, read status.traffic "
            "(describe | scripts/serving-revision.sh) to confirm the previous revision serves"
        )
    exit_5 = [c for line, c in body if _EXIT_5.match(line)]
    if not exit_5:
        found.append(f"{name}: rollback: a rollback that did not restore the previous revision must `exit 5` inside the rollback function")
    elif not any(
        "::error::" in line and "rollback did not restore" in line and c == exit_5[0] for line, c in body
    ):
        found.append(f"{name}: rollback: exit 5 must follow an ::error:: `rollback did not restore <prev>` in the same arm")
    if any(_EXIT_5.match(line) and not any(str(x) == "fn rollback" for x in c) for line, c in lines):
        found.append(f"{name}: rollback: exit 5 belongs to the rollback function (the check's own exit code is $RC)")
    return found


def refused_arm(lines, refused):
    """The command lines of the arm that follows the `--refused` call: its failure branch."""
    base = len(lines[refused][1])
    arm = []
    for line, conditions in lines[refused + 1 :]:
        if len(conditions) <= base:
            break
        arm.append((line, conditions))
    return arm


def check_refused_rollback(name, lines, sites):
    """The bare *.run.app answering (`--refused` failing) fails the job and moves NO traffic (D13).

    Both revisions boot from the same env: a rollback restores nothing. The
    failure arm is an `::error::` naming the missing `ORIGIN_SECRET` env of the
    service, then `exit "$RC"`; no rollback is called in it, nor after it.
    """
    refused = next(
        (at for at, (line, _) in enumerate(lines) if "rollout-check.sh" in line and "--refused" in line),
        None,
    )
    if refused is None:
        return []
    found = []
    if any(at > refused for at, _, _ in sites):
        found.append(f"{name}: refused: a bare origin that answers must fail the job with no rollback (both revisions boot from the same env)")
    arm = refused_arm(lines, refused)
    if not any("::error::" in line for line, _ in arm):
        found.append(f"{name}: refused: a bare origin that answers must end in an ::error:: annotation")
    elif not any("::error::" in line and "ORIGIN_SECRET" in line for line, _ in arm):
        found.append(f"{name}: refused: the ::error:: must name the service's missing ORIGIN_SECRET env")
    if not any(_RC_EXIT.match(line) for line, _ in arm):
        found.append(f"{name}: refused: the arm of a bare origin that answers must go on to `exit \"$RC\"`")
    return found


def check_worker_500(name, lines):
    """Exit 4, the Worker's own 500, is its own arm: an error naming the remedy, then the code."""
    arm = [(line, c) for line, c in lines if any(WORKER_500.search(x) for x in c)]
    if not arm:
        return [f"{name}: rollout-check.sh --via-worker: exit 4 (the Worker answered 500) needs its own arm, with no rollback"]
    found = []
    if not any("::error::" in line and "origin-secret push" in line for line, _ in arm):
        found.append(f"{name}: rollout-check.sh --via-worker: exit 4 must end in an ::error:: naming `origin-secret push`")
    if not any(_RC_EXIT.match(line) for line, _ in arm):
        found.append(f"{name}: rollout-check.sh --via-worker: the exit 4 arm must go on to `exit \"$RC\"`")
    return found


def check_rollback(name, doc):
    """A refused public check (exit 1) puts the traffic back on the recorded revision (D11).

    The rollback must sit inside the arm whose condition is the refusal exit
    code, and in no arm of another code (a Cloudflare challenge says nothing
    about the service): checked on the arms, not on where text appears.
    """
    step = next((s for s in steps_of(doc) if "--via-worker" in s.get("run", "")), None)
    if step is None:
        return []
    lines = shell_contexts(step["run"])
    sites = rollback_sites(lines)
    if not sites:
        return [f"{name}: rollback: a failed --via-worker check must move the traffic back (update-traffic --to-revisions)"]
    found = []
    found += check_rollback_failure(name, lines, sites)
    found += check_worker_500(name, lines)
    found += check_rollback_outcome(name, lines)
    found += check_refused_rollback(name, lines, sites)
    for _, _, conditions in sites:
        if any(CHALLENGE.search(c) for c in conditions):
            found.append(f"{name}: rollback: a Cloudflare challenge (exit 3) must not move the traffic back, only a failure")
        elif any(WORKER_500.search(c) for c in conditions):
            found.append(f"{name}: rollback: a Worker 500 (exit 4) must not move the traffic back: the service is not at fault, run origin-secret push")
        elif not any(REFUSAL.search(c) for c in conditions):
            found.append(f"{name}: rollback: the traffic goes back only in the arm whose condition is the refusal exit code (rollout-check.sh exit 1)")
    if "outputs.serving" not in str((step.get("env") or {}).values()):
        found.append(f"{name}: rollback: the revision to go back to is the recorded one (steps.<id>.outputs.serving)")
    return found


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
TEST_GLOBS = ("scripts/*_test.py", "scripts/*.test.sh", "infra/scripts/*.test.sh", "infra/worker/*.test.mjs")


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


def check_deploy(name, doc):
    found = []
    group = str(doc.get("concurrency", {}).get("group", ""))
    if "deploy-" not in group or "github.ref" not in group:
        found.append(f"{name}: concurrency: a group per environment (deploy-<env> derived from github.ref) is required")
    if doc.get("concurrency", {}).get("cancel-in-progress") is not False:
        found.append(f"{name}: concurrency: cancel-in-progress must be false (never cancel a rollout)")
    for rule in (
        check_build_args,
        check_no_set_env_vars,
        check_deploy_writes_release,
        check_skip_compares_release,
        check_gate_release,
        check_serving_skip,
        check_rollback,
        check_rollout_calls,
    ):
        found += rule(name, doc)
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
