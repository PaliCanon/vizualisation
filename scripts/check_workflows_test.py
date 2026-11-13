"""Tests for scripts/check_workflows.py (task 005, slice S5).

Each rule has a workflow that breaks it (the checker names the rule) and the
real workflows pass. Run: python3 scripts/check_workflows_test.py

From search-front/scripts/check_workflows_test.py. What differs: the
checkout is `vizualisation/` and the sibling ui-lib alone; two rules of our
own, each with its tests: a job that uses a local composite action running
cargo needs the siblings too (`SiblingsThroughAnAction`: the React site's
job built the engine through such an action until task 008), and every test file of
the repository is run by a step of ci.yml (`EveryTestFileHasItsStep`, the
task's S5 row).

deploy.yml only builds the image and pushes it to GHCR (prod, on the lsh-tech
server, pulls it by digest): its fixture is `GOOD_DEPLOY`, and each rule of
`check_deploy` has its class below, with one mutation of the fixture that
breaks it: no GCP (`NoGcp`), the job's permissions (`ImageJobPermissions`),
the login (`GhcrLogin`), the image refs and tags (`ImageRefs`), the source
label (`SourceLabel`), the existence check (`ExistenceCheck`), the digest in
the job summary (`DigestSummary`), and no build args (`NoBuildArgs`).
"""

import pathlib
import re
import sys
import unittest

import yaml

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import check_workflows as cw  # noqa: E402

GOOD_DEPLOY = """
name: deploy
on:
  push:
    tags: ["v*"]
concurrency:
  group: deploy-prod
  cancel-in-progress: false
permissions:
  contents: read
jobs:
  image:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v7
        with:
          persist-credentials: false
      - name: vars
        id: vars
        run: |
          set -euo pipefail
          TAG=$(scripts/image-hash.sh ../ui-lib)
          IMAGE=ghcr.io/palicanon/vizualisation
          echo "tag=$TAG" >> "$GITHUB_OUTPUT"
          echo "image=$IMAGE" >> "$GITHUB_OUTPUT"
          echo "image_hash=$IMAGE:$TAG" >> "$GITHUB_OUTPUT"
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/setup-buildx-action@v3
      - name: check
        id: check-image
        env:
          IMAGE_HASH: ${{ steps.vars.outputs.image_hash }}
        run: |
          set -euo pipefail
          # the image exists when its manifest can be read
          if docker buildx imagetools inspect "$IMAGE_HASH" >/dev/null 2>&1; then
            echo "exists=true" >> "$GITHUB_OUTPUT"
          else
            echo "exists=false" >> "$GITHUB_OUTPUT"
          fi
      - uses: docker/build-push-action@v6
        if: steps.check-image.outputs.exists != 'true'
        with:
          push: true
          tags: |
            ${{ steps.vars.outputs.image_hash }}
            ${{ steps.vars.outputs.image }}:${{ github.ref_name }}
          labels: |
            org.opencontainers.image.source=https://github.com/${{ github.repository }}
      - name: release tag
        if: steps.check-image.outputs.exists == 'true'
        env:
          IMAGE: ${{ steps.vars.outputs.image }}
          IMAGE_HASH: ${{ steps.vars.outputs.image_hash }}
          REF: ${{ github.ref_name }}
        run: docker buildx imagetools create -t "$IMAGE:$REF" "$IMAGE_HASH"
      - name: summary
        env:
          IMAGE: ${{ steps.vars.outputs.image }}
          IMAGE_HASH: ${{ steps.vars.outputs.image_hash }}
        run: |
          set -euo pipefail
          DIGEST=$(docker buildx imagetools inspect "$IMAGE_HASH" --format '{{json .Manifest}}' | jq -r .digest)
          test -n "$DIGEST"
          echo "digest: $IMAGE@$DIGEST" >> "$GITHUB_STEP_SUMMARY"
"""


def parse(text):
    return yaml.safe_load(text)


def violations(deploy=GOOD_DEPLOY, extra=None, actions=None):
    workflows = {"deploy.yml": parse(deploy)}
    for name, text in (extra or {}).items():
        workflows[name] = parse(text)
    return cw.check(workflows, {n: parse(t) for n, t in (actions or {}).items()})


def mutate(old, new):
    assert GOOD_DEPLOY.count(old) >= 1, old
    return GOOD_DEPLOY.replace(old, new)


def only(found, tag):
    """The violations of one rule (their text holds `tag:`)."""
    return [v for v in found if f" {tag}:" in v]


class RealDeploy(unittest.TestCase):
    """The real deploy.yml: a tag builds the image and pushes it to GHCR."""

    def setUp(self):
        self.path = pathlib.Path(__file__).parent.parent / ".github" / "workflows" / "deploy.yml"
        self.text = self.path.read_text()
        self.doc = yaml.safe_load(self.text)
        self.code = "\n".join(l.split("#", 1)[0] for l in self.text.splitlines())

    def test_only_a_tag_triggers_it(self):
        push = self.doc[True]["push"]  # YAML 1.1 reads the key `on` as True
        self.assertNotIn("branches", push)
        self.assertEqual(push["tags"], ["v*"])
        self.assertEqual(list(self.doc[True]), ["push"])

    def test_one_deploy_at_a_time_and_never_cancelled(self):
        self.assertEqual(self.doc["concurrency"], {"group": "deploy-prod", "cancel-in-progress": False})

    def test_it_has_no_environment_variables_of_the_old_deploy(self):
        for key in ("env", "vars"):
            self.assertNotIn(key, self.doc)
        for needle in ("REGION", "PROJECT", "HOST", "WIF_PROVIDER", "DEPLOYER_SA", "GCP_PROJECT"):
            self.assertNotIn(needle, self.code, needle)

    def test_nothing_of_gcp_cloudflare_or_the_rollout_is_left(self):
        for needle in ("gcloud", "google-github-actions", "docker.pkg.dev", "run.app", "id-token",
                       "x-origin-auth", "ORIGIN_SECRET", "rollout-check", "browser-actions", "Worker"):
            self.assertNotIn(needle, self.code, needle)

    def test_the_one_job_pushes_to_ghcr(self):
        self.assertEqual(list(self.doc["jobs"]), ["image"])
        self.assertEqual(self.doc["permissions"], {"contents": "read"})
        self.assertEqual(self.doc["jobs"]["image"]["permissions"], {"contents": "read", "packages": "write"})
        self.assertIn("ghcr.io/palicanon/vizualisation", self.code)

    def test_the_summary_pins_the_digest(self):
        self.assertIn("GITHUB_STEP_SUMMARY", self.code)
        self.assertIn("@$DIGEST", self.code)


class Rules(unittest.TestCase):
    def test_a_good_deploy_passes(self):
        self.assertEqual(violations(), [])

    def test_a_secret_in_a_run_line_is_refused(self):
        bad = mutate('echo "tag=$TAG" >> "$GITHUB_OUTPUT"',
                     'echo "tag=${{ secrets.TOKEN }}" >> "$GITHUB_OUTPUT"')
        self.assertTrue(any("secret in a run" in v for v in violations(bad)))

    def test_a_secret_in_a_with_is_allowed(self):
        ok = mutate("push: true", "push: true\n          token: ${{ secrets.TOKEN }}")
        self.assertEqual(violations(ok), [])

    def test_a_write_permission_other_than_packages_is_refused(self):
        bad = mutate("contents: read", "contents: write")
        self.assertTrue(any("permissions" in v and "more than read" in v for v in violations(bad)))

    def test_a_job_write_permission_other_than_packages_is_refused(self):
        bad = mutate("      contents: read\n      packages: write", "      contents: read\n      packages: write\n      issues: write")
        self.assertTrue(any("permissions" in v and "issues" in v for v in violations(bad)))

    def test_id_token_write_is_no_longer_allowed(self):
        bad = mutate("permissions:\n  contents: read\njobs:", "permissions:\n  contents: read\n  id-token: write\njobs:")
        self.assertTrue(any("permissions" in v and "id-token" in v for v in violations(bad)))

    def test_packages_write_is_allowed_where_the_job_needs_it(self):
        self.assertEqual([v for v in violations() if "permissions" in v], [])

    def test_missing_permissions_are_refused(self):
        bad = mutate("permissions:\n  contents: read\njobs:", "jobs:")
        self.assertTrue(any("permissions" in v for v in violations(bad)))

    def test_the_prod_concurrency_group_is_required(self):
        bad = mutate("group: deploy-prod", "group: deploy")
        self.assertTrue(any("concurrency" in v for v in violations(bad)))

    def test_a_deploy_must_not_cancel_a_running_build(self):
        bad = mutate("cancel-in-progress: false", "cancel-in-progress: true")
        self.assertTrue(any("concurrency" in v for v in violations(bad)))

    def test_a_build_secret_is_refused(self):
        bad = mutate("push: true", "push: true\n          secrets: gh_token=${{ steps.t.outputs.token }}")
        self.assertTrue(any("build-push" in v for v in violations(bad)))

    def test_an_ssh_forward_is_refused(self):
        bad = mutate("push: true", "push: true\n          ssh: default")
        self.assertTrue(any("build-push" in v for v in violations(bad)))

    def test_a_checkout_must_not_persist_credentials(self):
        bad = mutate("          persist-credentials: false\n", "")
        self.assertTrue(any("persist-credentials" in v for v in violations(bad)))

    def test_an_unversioned_action_is_refused(self):
        bad = mutate("actions/checkout@v7", "actions/checkout")
        self.assertTrue(any("version" in v for v in violations(bad)))

    def test_the_action_is_checked_too(self):
        action = """
runs:
  using: composite
  steps:
    - uses: actions/checkout@v7
    - shell: bash
      run: echo ${{ secrets.X }}
"""
        found = violations(actions={"siblings": action})
        self.assertTrue(any("persist-credentials" in v for v in found))
        self.assertTrue(any("secret in a run" in v for v in found))


class NoBuildArgs(unittest.TestCase):
    """The image takes no build args: the release is not in the image, so its hash does not change with the ref."""

    def test_the_build_has_no_build_args(self):
        # R1, the design sentence: "No build args: the release is not in the
        # image, so its hash does not change with the ref" (016, 060).
        bad = mutate("push: true", "push: true\n          build-args: |\n            X=1")
        self.assertTrue(any("takes no build args" in v for v in violations(bad)), violations(bad))

    def test_the_old_branch_and_sha_build_args_are_refused(self):
        old = "push: true\n          build-args: |\n            BUILD_BRANCH=${{ github.ref_name }}\n            BUILD_SHA=${{ github.sha }}"
        bad = mutate("push: true", old)
        self.assertTrue(any("takes no build args" in v for v in violations(bad)), violations(bad))

    def test_an_empty_build_args_key_is_still_refused(self):
        bad = mutate("push: true", "push: true\n          build-args: ''")
        self.assertTrue(any("takes no build args" in v for v in violations(bad)), violations(bad))


class NoGcp(unittest.TestCase):
    """(a) Nothing of Google Cloud is left: prod is the lsh-tech server, the registry is GHCR."""

    def refused(self, bad, needle):
        found = only(violations(bad), "no-gcp")
        self.assertTrue(any(needle in v for v in found), found)

    def test_gcloud_in_a_run_is_refused(self):
        self.refused(mutate("test -n \"$DIGEST\"", "gcloud auth list"), "gcloud")

    def test_a_google_github_action_is_refused(self):
        bad = mutate("      - uses: docker/setup-buildx-action@v3\n",
                     "      - uses: google-github-actions/auth@v2\n      - uses: docker/setup-buildx-action@v3\n")
        self.refused(bad, "google-github-actions")

    def test_an_artifact_registry_ref_is_refused(self):
        self.refused(mutate("          echo \"tag=$TAG\"", "          echo europe-west1-docker.pkg.dev\n          echo \"tag=$TAG\""), "docker.pkg.dev")

    def test_a_cloud_run_url_is_refused(self):
        self.refused(mutate("test -n \"$DIGEST\"", "echo https://x.run.app"), "run.app")

    def test_the_good_deploy_names_none(self):
        self.assertEqual(only(violations(), "no-gcp"), [])


class ImageJobPermissions(unittest.TestCase):
    """(b) The job that builds has `packages: write` and `contents: read`, and no `id-token`."""

    JOB = "    permissions:\n      contents: read\n      packages: write\n"

    def found(self, bad):
        return only(violations(bad), "job-permissions")

    def test_the_build_job_needs_packages_write(self):
        found = self.found(mutate(self.JOB, "    permissions:\n      contents: read\n"))
        self.assertTrue(any("packages: write" in v for v in found), found)

    def test_a_build_job_without_permissions_is_refused(self):
        found = self.found(mutate(self.JOB, ""))
        self.assertTrue(any("packages: write" in v for v in found), found)

    def test_the_build_job_needs_contents_read(self):
        found = self.found(mutate(self.JOB, "    permissions:\n      packages: write\n"))
        self.assertTrue(any("contents: read" in v for v in found), found)

    def test_the_build_job_has_no_id_token(self):
        found = self.found(mutate(self.JOB, self.JOB + "      id-token: none\n"))
        self.assertTrue(any("id-token" in v for v in found), found)

    def test_packages_read_is_not_enough(self):
        found = self.found(mutate("packages: write", "packages: read"))
        self.assertTrue(any("packages: write" in v for v in found), found)

    def test_the_good_deploy_names_none(self):
        self.assertEqual(self.found(GOOD_DEPLOY), [])


class GhcrLogin(unittest.TestCase):
    """(c) A docker/login-action on ghcr.io with the actor and the GITHUB_TOKEN, before the build, and no other registry."""

    STEP = (
        "      - uses: docker/login-action@v3\n"
        "        with:\n"
        "          registry: ghcr.io\n"
        "          username: ${{ github.actor }}\n"
        "          password: ${{ secrets.GITHUB_TOKEN }}\n"
    )

    def found(self, bad):
        return only(violations(bad), "login")

    def test_the_step_is_in_the_good_fixture(self):
        self.assertIn(self.STEP, GOOD_DEPLOY)

    def test_a_build_without_a_login_is_refused(self):
        found = self.found(mutate(self.STEP, ""))
        self.assertTrue(any("ghcr.io" in v for v in found), found)

    def test_the_registry_must_be_ghcr(self):
        found = self.found(mutate("registry: ghcr.io", "registry: docker.io"))
        self.assertTrue(any("registry" in v for v in found), found)

    def test_a_login_without_a_registry_goes_to_docker_hub(self):
        found = self.found(mutate("          registry: ghcr.io\n", ""))
        self.assertTrue(any("registry" in v for v in found), found)

    def test_the_username_is_the_actor(self):
        found = self.found(mutate("username: ${{ github.actor }}", "username: someone"))
        self.assertTrue(any("username" in v for v in found), found)

    def test_the_password_is_the_github_token(self):
        found = self.found(mutate("password: ${{ secrets.GITHUB_TOKEN }}", "password: ${{ secrets.PAT }}"))
        self.assertTrue(any("password" in v for v in found), found)

    def test_the_login_comes_before_the_build(self):
        late = mutate(self.STEP, "").replace(
            "      - name: release tag\n", self.STEP + "      - name: release tag\n", 1)
        found = self.found(late)
        self.assertTrue(any("before" in v for v in found), found)

    def test_a_second_login_to_another_registry_is_refused(self):
        other = self.STEP.replace("ghcr.io", "registry.example.com")
        found = self.found(mutate(self.STEP, self.STEP + other))
        self.assertTrue(any("registry.example.com" in v for v in found), found)

    def test_a_docker_login_in_a_run_is_refused(self):
        found = self.found(mutate("test -n \"$DIGEST\"", "docker login ghcr.io -u x --password-stdin"))
        self.assertTrue(any("docker login" in v for v in found), found)

    def test_the_good_deploy_names_none(self):
        self.assertEqual(self.found(GOOD_DEPLOY), [])


class ImageRefs(unittest.TestCase):
    """(d) The image is ghcr.io/palicanon/..., tagged with the `h-` hash and with the release tag (github.ref_name)."""

    HASH = "            ${{ steps.vars.outputs.image_hash }}\n"
    RELEASE = "            ${{ steps.vars.outputs.image }}:${{ github.ref_name }}\n"

    def found(self, bad):
        return only(violations(bad), "image-refs")

    def test_the_tags_are_in_the_good_fixture(self):
        self.assertIn(self.HASH + self.RELEASE, GOOD_DEPLOY)

    def test_the_image_is_under_ghcr_palicanon(self):
        found = self.found(mutate("IMAGE=ghcr.io/palicanon/vizualisation", "IMAGE=ghcr.io/someone/vizualisation"))
        self.assertTrue(any("ghcr.io/palicanon/" in v for v in found), found)

    def test_the_image_is_not_another_registry(self):
        found = self.found(mutate("IMAGE=ghcr.io/palicanon/vizualisation", "IMAGE=docker.io/palicanon/vizualisation"))
        self.assertTrue(any("ghcr.io/palicanon/" in v for v in found), found)

    def test_the_build_is_tagged_with_the_hash(self):
        found = self.found(mutate(self.HASH, ""))
        self.assertTrue(any("hash tag" in v for v in found), found)

    def test_the_build_is_tagged_with_the_release(self):
        found = self.found(mutate(self.RELEASE, ""))
        self.assertTrue(any("github.ref_name" in v for v in found), found)

    def test_the_release_tag_is_the_ref_not_the_commit(self):
        found = self.found(mutate(":${{ github.ref_name }}\n", ":${{ github.sha }}\n"))
        self.assertTrue(any("github.ref_name" in v for v in found), found)

    def test_a_build_with_no_tags_is_refused(self):
        found = self.found(mutate(self.HASH + self.RELEASE, "            ghcr.io/palicanon/vizualisation:latest\n"))
        self.assertEqual(len(found), 2, found)

    def test_the_hash_is_computed_by_the_script(self):
        found = self.found(mutate("TAG=$(scripts/image-hash.sh ../ui-lib)", "TAG=latest"))
        self.assertTrue(any("image-hash.sh" in v for v in found), found)

    def test_the_good_deploy_names_none(self):
        self.assertEqual(self.found(GOOD_DEPLOY), [])


class SourceLabel(unittest.TestCase):
    """(e) The image carries `org.opencontainers.image.source=`, which links the package to the repository."""

    LABELS = (
        "          labels: |\n"
        "            org.opencontainers.image.source=https://github.com/${{ github.repository }}\n"
    )

    def found(self, bad):
        return only(violations(bad), "labels")

    def test_a_build_without_labels_is_refused(self):
        found = self.found(mutate(self.LABELS, ""))
        self.assertTrue(any("org.opencontainers.image.source=" in v for v in found), found)

    def test_another_label_does_not_link_the_package(self):
        found = self.found(mutate("org.opencontainers.image.source=", "org.opencontainers.image.title="))
        self.assertTrue(any("org.opencontainers.image.source=" in v for v in found), found)

    def test_the_good_deploy_names_none(self):
        self.assertEqual(self.found(GOOD_DEPLOY), [])


class ExistenceCheck(unittest.TestCase):
    """(f) `docker buildx imagetools inspect` tells whether the image exists, and the build is skipped when it does."""

    INSPECT = 'docker buildx imagetools inspect "$IMAGE_HASH" >/dev/null 2>&1'
    BUILDX = "      - uses: docker/setup-buildx-action@v3\n"

    def found(self, bad):
        return only(violations(bad), "existence")

    def test_the_check_reads_the_manifest_with_buildx(self):
        found = self.found(mutate(self.INSPECT, 'docker manifest inspect "$IMAGE_HASH" >/dev/null 2>&1'))
        self.assertTrue(any("imagetools inspect" in v for v in found), found)

    def test_the_check_is_not_only_a_comment(self):
        found = self.found(mutate(self.INSPECT, "false"))
        self.assertTrue(any("imagetools inspect" in v for v in found), found)

    def test_the_build_is_skipped_when_the_image_exists(self):
        found = self.found(mutate("        if: steps.check-image.outputs.exists != 'true'\n", ""))
        self.assertTrue(any("exists != 'true'" in v for v in found), found)

    def test_the_skip_reads_the_output_of_the_check(self):
        found = self.found(mutate("steps.check-image.outputs.exists != 'true'", "steps.vars.outputs.exists != 'true'"))
        self.assertTrue(any("exists != 'true'" in v for v in found), found)

    def test_the_skip_is_not_inverted(self):
        bad = mutate("        if: steps.check-image.outputs.exists != 'true'\n", "        if: steps.check-image.outputs.exists == 'true'\n")
        found = self.found(bad)
        self.assertTrue(any("exists != 'true'" in v for v in found), found)

    def test_buildx_is_set_up_before_the_check(self):
        late = mutate(self.BUILDX, "").replace(
            "      - uses: docker/build-push-action@v6\n", self.BUILDX + "      - uses: docker/build-push-action@v6\n", 1)
        found = self.found(late)
        self.assertTrue(any("setup-buildx-action" in v and "before" in v for v in found), found)

    def test_buildx_is_set_up_unconditionally(self):
        bad = mutate(self.BUILDX, self.BUILDX + "        if: steps.vars.outputs.tag != ''\n")
        found = self.found(bad)
        self.assertTrue(any("setup-buildx-action" in v and "if" in v for v in found), found)

    def test_buildx_must_be_set_up(self):
        found = self.found(mutate(self.BUILDX, ""))
        self.assertTrue(any("setup-buildx-action" in v for v in found), found)

    def test_the_good_deploy_names_none(self):
        self.assertEqual(self.found(GOOD_DEPLOY), [])


class DigestSummary(unittest.TestCase):
    """(g) A step after the build writes the image's digest to `$GITHUB_STEP_SUMMARY`: what the stack is updated with."""

    LINE = '          echo "digest: $IMAGE@$DIGEST" >> "$GITHUB_STEP_SUMMARY"\n'

    def found(self, bad):
        return only(violations(bad), "summary")

    def test_a_deploy_without_a_summary_is_refused(self):
        found = self.found(mutate(self.LINE, ""))
        self.assertTrue(any("GITHUB_STEP_SUMMARY" in v for v in found), found)

    def test_the_summary_must_hold_the_digest(self):
        found = self.found(mutate('echo "digest: $IMAGE@$DIGEST" >> ', 'echo "built" >> '))
        self.assertTrue(any("digest" in v for v in found), found)

    def test_the_digest_is_pinned_to_the_image_with_an_at_sign(self):
        found = self.found(mutate("$IMAGE@$DIGEST", "$IMAGE:$DIGEST"))
        self.assertTrue(any("@" in v for v in found), found)

    def test_the_summary_comes_after_the_build(self):
        at = GOOD_DEPLOY.index("      - name: summary\n")
        summary = GOOD_DEPLOY[at:]
        build = "      - uses: docker/build-push-action@v6\n"
        early = GOOD_DEPLOY[:at].replace(build, summary + build, 1)
        found = self.found(early)
        self.assertTrue(any("after" in v for v in found), found)

    def test_the_good_deploy_names_none(self):
        self.assertEqual(self.found(GOOD_DEPLOY), [])


class NodeTestArguments(unittest.TestCase):
    """Task 007 F3: `node --test infra/worker/` passed on node 26 and failed
    on the runner's node 22 ("Cannot find module .../infra/worker"): node 22
    runs a directory argument as a module. The arguments are quoted globs."""

    def found(self, command):
        ci = "name: ci\non: push\npermissions:\n  contents: read\njobs:\n  j:\n    runs-on: x\n    steps:\n      - run: " + command + "\n"
        return violations(extra={"ci.yml": ci})

    def test_a_directory_argument_is_refused(self):
        for command in ("node --test infra/worker/", "node --test infra/worker", "node --test ."):
            found = self.found(command)
            self.assertTrue(any("node --test" in v for v in found), (command, found))

    def test_a_quoted_glob_passes(self):
        self.assertEqual(self.found("node --test 'infra/worker/*.test.mjs'"), [])
        self.assertEqual(self.found('node --test "infra/worker/*.test.mjs"'), [])

    def test_a_file_passes(self):
        self.assertEqual(self.found("node --test infra/worker/proxy.test.mjs"), [])


class SiblingsForCargo(unittest.TestCase):
    """search-front 007 F4: the fmt job ran `cargo fmt --all --check` without the
    sibling checkout; rustfmt follows `app/build.rs`'s `#[path]` into
    `../../ui-lib/build/placeholders.rs` and failed. Every job that runs
    cargo has the `siblings` action."""

    JOB = (
        "name: ci\non: push\npermissions:\n  contents: read\njobs:\n  j:\n    runs-on: x\n    steps:\n"
        "      - uses: actions/checkout@v7\n        with:\n          persist-credentials: false\n%s"
    )

    def found(self, steps):
        return violations(extra={"ci.yml": self.JOB % steps})

    def test_a_cargo_job_without_the_siblings_is_refused(self):
        found = self.found("      - run: cargo fmt --all --check\n")
        self.assertTrue(any("siblings" in v and "j" in v for v in found), found)

    def test_a_cargo_job_with_the_siblings_passes(self):
        steps = "      - uses: ./vizualisation/.github/actions/siblings\n      - run: cargo fmt --all --check\n"
        self.assertEqual(self.found(steps), [])

    def test_a_job_without_cargo_needs_no_siblings(self):
        self.assertEqual(self.found("      - run: node --test 'infra/worker/*.test.mjs'\n"), [])

    def test_the_fmt_job_of_ci_has_the_siblings(self):
        text = (pathlib.Path(__file__).resolve().parent.parent / ".github/workflows/ci.yml").read_text()
        fmt = parse(text)["jobs"]["fmt"]
        self.assertTrue(any(str(step.get("uses", "")).endswith("/siblings") for step in fmt["steps"]))


class SiblingsThroughAnAction(unittest.TestCase):
    """A job may run cargo inside a local composite action (the React site's
    job did, `.github/actions/build`, until task 008): the workspace's
    [patch] reaches ../ui-lib there too, so a job using a local action that
    runs cargo needs the siblings. The actions below are made up."""

    BUILD = """
runs:
  using: composite
  steps:
    - shell: bash
      run: cargo build -p map-engine --target wasm32-unknown-unknown
"""
    NO_CARGO = """
runs:
  using: composite
  steps:
    - shell: bash
      run: wasm-bindgen --version
"""
    JOB = SiblingsForCargo.JOB

    def found(self, steps, action):
        return violations(extra={"ci.yml": self.JOB % steps}, actions={"build/action.yml": action})

    def test_a_job_using_a_cargo_action_without_the_siblings_is_refused(self):
        found = self.found("      - uses: ./vizualisation/.github/actions/build\n", self.BUILD)
        self.assertTrue(any("siblings" in v and "j" in v for v in found), found)

    def test_with_the_siblings_it_passes(self):
        steps = "      - uses: ./vizualisation/.github/actions/siblings\n      - uses: ./vizualisation/.github/actions/build\n"
        self.assertEqual(self.found(steps, self.BUILD), [])

    def test_an_action_without_cargo_needs_no_siblings(self):
        self.assertEqual(self.found("      - uses: ./vizualisation/.github/actions/build\n", self.NO_CARGO), [])


class EveryTestFileHasItsStep(unittest.TestCase):
    """Every test file (scripts/*_test.py, scripts/*.test.sh,
    infra/scripts/*.test.sh, infra/worker/*.test.mjs) is run by a step of
    ci.yml: a test no job runs guards nothing (task 005, S5)."""

    FILES = ["infra/worker/proxy.test.mjs", "scripts/a_test.py", "scripts/b.test.sh"]

    def found(self, runs):
        ci = parse("name: ci\non: push\npermissions:\n  contents: read\njobs:\n  j:\n    runs-on: x\n    steps:\n"
                   + "".join(f"      - run: {r}\n" for r in runs))
        return cw.check_test_steps("ci.yml", ci, self.FILES)

    def test_each_file_run_by_a_step_passes(self):
        runs = ["python3 scripts/a_test.py", "bash scripts/b.test.sh", "node --test 'infra/worker/*.test.mjs'"]
        self.assertEqual(self.found(runs), [])

    def test_a_file_no_step_runs_is_named(self):
        found = self.found(["python3 scripts/a_test.py", "node --test 'infra/worker/*.test.mjs'"])
        self.assertEqual(len(found), 1, found)
        self.assertIn("scripts/b.test.sh", found[0])

    def test_a_quoted_glob_covers_its_files(self):
        found = self.found(["python3 scripts/a_test.py", 'bash "scripts/b.test.sh"', 'node --test "infra/worker/*.test.mjs"'])
        self.assertEqual(found, [])

    def test_a_mention_in_a_comment_does_not_count(self):
        found = self.found(["python3 scripts/a_test.py # and scripts/b.test.sh", "node --test 'infra/worker/*.test.mjs'"])
        self.assertTrue(any("scripts/b.test.sh" in v for v in found), found)

    def test_the_repository_test_files_are_found(self):
        root = pathlib.Path(__file__).resolve().parent.parent
        files = cw.test_files(root)
        self.assertIn("scripts/check_workflows_test.py", files)
        self.assertIn("scripts/manifest-ref.test.sh", files)

    def test_a_repository_whose_ci_runs_no_test_is_refused(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / ".github" / "workflows").mkdir(parents=True)
            (root / "scripts").mkdir()
            (root / "scripts" / "x_test.py").write_text("")
            (root / ".github" / "workflows" / "ci.yml").write_text(
                "name: ci\non: push\npermissions:\n  contents: read\njobs:\n  j:\n    runs-on: x\n    steps:\n      - run: echo\n"
            )
            found = cw.check_repository(root)
        self.assertTrue(any("scripts/x_test.py" in v for v in found), found)


class Loading(unittest.TestCase):
    def test_a_file_that_does_not_parse_is_reported_not_raised(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root / ".github" / "workflows").mkdir(parents=True)
            (root / ".github" / "workflows" / "ci.yml").write_text("name: a: b: c\n")
            found = cw.check_repository(root)
        self.assertTrue(any("ci.yml" in v and "YAML" in v for v in found), found)

    def test_no_workflow_at_all_is_reported(self):
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            self.assertTrue(cw.check_repository(pathlib.Path(tmp)))


class RealFiles(unittest.TestCase):
    def test_the_repository_workflows_pass(self):
        found = cw.check_repository(pathlib.Path(__file__).resolve().parent.parent)
        self.assertEqual(found, [])


if __name__ == "__main__":
    unittest.main()
