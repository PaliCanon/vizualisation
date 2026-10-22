#!/usr/bin/env bash
# Tests for the Cloud Run service's lifecycle (task 016): tofu must ignore the
# container env, because deploy.yml writes VIZ_ENV and VIZ_SHA
# with `gcloud run deploy --update-env-vars`; a later `tofu apply` that rewrote
# the template would flip the badge back to "dev" with no diff shown. The list
# is read on the `ignore_changes = [` line itself, never from comments, and the
# README must carry the apply order (the ignore is applied BEFORE the first
# deploy that writes the env).
# Not caught: an `env` entry on that line that sits in a different lifecycle
# block (the file holds one such line today, asserted below), or a README
# sentence that says the order but is wrong about it (only its wording is read).
# Run: bash infra/scripts/lifecycle.test.sh
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
gcp="${GCP_TF:-$here/../gcp.tf}"
readme="${README_MD:-$here/../README.md}"
# A hook is a local mutation tool: set in CI it would make this suite test a
# copy, so it is refused (016, H5).
for hook in GCP_TF README_MD; do
  if [ -n "${CI:-}" ] && [ -n "${!hook:-}" ]; then
    echo "$(basename "$0"): the test hook $hook is refused when CI is set (it points the suite at a copy)" >&2
    exit 64
  fi
done

pass=0
fail=0
ok() { pass=$((pass + 1)); echo "ok   $1"; }
ko() { fail=$((fail + 1)); echo "FAIL $1"; }
check() { # check "name" command...   (passes when the command succeeds)
  local name=$1
  shift
  if "$@"; then ok "$name"; else ko "$name"; fi
}

lines=$(grep -E '^[[:space:]]*ignore_changes[[:space:]]*=[[:space:]]*\[' "$gcp")
check "gcp.tf: exactly one ignore_changes list" test "$(printf '%s\n' "$lines" | grep -c .)" -eq 1

# The entries of the list, one per line, trimmed.
entries=$(printf '%s\n' "$lines" | sed -e 's/^[^=]*=[[:space:]]*\[//' -e 's/\][[:space:]]*$//' | tr ',' '\n' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
has_entry() { printf '%s\n' "$entries" | grep -Fxq -- "$1"; }

for entry in \
  'template[0].containers[0].image' \
  'template[0].revision' \
  'scaling' \
  'client' \
  'client_version' \
  'template[0].containers[0].env'; do
  check "gcp.tf: ignore_changes holds $entry" has_entry "$entry"
done

# The brief of rollout-check.test.sh reads the first line of the list: it stays on one line.
check "gcp.tf: the list is closed on its own line" bash -c '[[ "$1" =~ \][[:space:]]*$ ]]' _ "$lines"

# The order is written above the list too.
comment=$(grep -B12 -E '^[[:space:]]*ignore_changes[[:space:]]*=[[:space:]]*\[' "$gcp" | grep -E '^[[:space:]]*#')
check "gcp.tf: the comment says the ignore is applied before the first deploy that writes the env" \
  bash -c '[[ "$1" =~ [Aa]pplied.*before\ the\ first\ deploy ]]' _ "$(printf '%s ' $comment | tr -s ' ')"

# README: the paragraph that says what tofu ignores.
para=$(awk '/^CI owns the image/{p=1} p&&/^$/{exit} p' "$readme" | tr '\n' ' ' | tr -s ' ')
check "README: the 'CI owns the image' paragraph is found" test -n "$para"
check "README: the ignored list names the container env" \
  bash -c '[[ "$1" =~ tofu\ ignores\ .*\`env\` ]]' _ "$para"
check "README: it says the apply must come before the first deploy that writes the env" \
  bash -c '[[ "$1" =~ applied.*before\ the\ first\ deploy ]]' _ "$para"
check "README: it names VIZ_ENV and VIZ_SHA" \
  bash -c '[[ "$1" == *VIZ_ENV* && "$1" == *VIZ_SHA* ]]' _ "$para"
check "README: it says how to change RUST_LOG or ORIGIN_SECRET afterwards" \
  bash -c '[[ "$1" == *RUST_LOG* && "$1" == *ORIGIN_SECRET* && "$1" == *-replace* ]]' _ "$para"
# After a `tofu apply -replace` the release env is gone: the README says so and
# says what restores it (016, like task 060).
check "README: a replace of the service loses the release env until the next push to main" \
  bash -c '[[ "$1" == *"until the next push to main"* && "$1" == *VIZ_ENV* ]]' _ "$para"
# A TAG release is restored by a by-hand gcloud run deploy; the next push to main writes
# 'main · <sha7>'. deploy.yml has NO workflow_dispatch (016, lead): the README must not
# promise a branch dispatch, and must say the deploy never uses --set-env-vars.
has() { bash -c '[[ "$1" == *"$2"* ]]' _ "$1" "$2"; }
# 016 review F2: after a replace a prod tag has no push to wait for: the by-hand command.
check "README: a tag release is restored by a by-hand gcloud run deploy of the tag's values" \
  has "$para" 'gcloud run deploy <service> --region europe-west1 --update-env-vars VIZ_ENV=<tag>,VIZ_SHA='
check "README: it says there is no manual trigger in this repo" \
  has "$para" 'no manual trigger'
check "README: the next push to main writes main and the sha7" \
  has "$para" '`main · <sha7>`'
check "README: it promises no branch dispatch (deploy.yml has none)" \
  bash -c '[[ "$1" != *"branch dispatch"* && "$1" != *"<branch> · <sha7>"* ]]' _ "$para"
check "README: it says the deploy never uses --set-env-vars" \
  bash -c '[[ "$1" == *"never"*"--set-env-vars"* ]]' _ "$para"
# deploy.yml really has no dispatch trigger: if one lands, this test says the README sentence must change.
check "deploy.yml has no workflow_dispatch" bash -c '! grep -q workflow_dispatch "$1"' _ "$here/../../.github/workflows/deploy.yml"

echo
echo "lifecycle.test.sh: $pass passed, $fail failed"
test "$fail" -eq 0
