#!/usr/bin/env bash
# From search-front/infra/scripts/origin-secret.test.sh. What differs: the
# name (vizualisation-origin-secret-prod) and the scope: dev and its Worker
# are gone (task 022), so there is no `push`, no Cloudflare call and no host
# check; the script only creates and checks prod's Cloud Run secret.
#
# Tests for infra/scripts/origin-secret: `gcloud` and `curl` are stubs first
# on PATH that log their argv and stdin. The contract under test: the secret
# value travels on stdin only (never in an argument, never on screen),
# `create` is idempotent, `check` prints OK or why not, nothing calls the
# network (curl) and only `prod` is an environment.
# Run: bash infra/scripts/origin-secret.test.sh
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
script="$here/origin-secret"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
bin="$work/bin"
log="$work/log"
mkdir -p "$bin" "$log"

VALUE="stubbed-origin-secret-value-0123456789abcdef"

# gcloud: `versions list` prints $STUB_VERSIONS; `versions access` prints
# the known value; `versions add` records its stdin.
cat >"$bin/gcloud" <<STUB
#!/usr/bin/env bash
printf '%s\n' "\$*" >>"$log/gcloud.argv"
case "\$*" in
  *"versions list"*) printf '%s' "\${STUB_VERSIONS:-}" ;;
  *"versions access"*) printf '%s' "$VALUE" ;;
  *"versions add"*) cat >"$log/gcloud.add.stdin" ;;
esac
STUB
# curl: logs argv and answers 200. The script must never call it.
cat >"$bin/curl" <<STUB
#!/usr/bin/env bash
printf '%s\n' "\$*" >>"$log/curl.argv"
printf '200'
STUB
chmod +x "$bin/gcloud" "$bin/curl"

pass=0
fail=0
ok() { pass=$((pass + 1)); echo "ok   $1"; }
ko() { fail=$((fail + 1)); echo "FAIL $1"; }
check() { # check "name" command...   (passes when the command succeeds)
  local name=$1
  shift
  if "$@"; then ok "$name"; else ko "$name"; fi
}

reset() { rm -f "$log"/*; : >"$log/gcloud.argv"; : >"$log/curl.argv"; }
# run <args...> → $out (stdout+stderr), $code. No Cloudflare variable is set:
# the script needs none.
run() {
  out=$(env -u CLOUDFLARE_API_TOKEN PATH="$bin:$PATH" "$script" "$@" 2>&1)
  code=$?
}
out_has() { printf '%s' "$out" | grep -q -- "$1"; }
out_is_ok() { [ "$out" = "OK" ]; }
refused() { [ "$code" -ne 0 ] && out_has "not OK" && { [ -z "${1:-}" ] || out_has "$1"; }; }
no_curl() { ! test -s "$log/curl.argv"; }
no_value_anywhere() {
  ! grep -qF "$VALUE" "$log/gcloud.argv" "$log/curl.argv" 2>/dev/null && ! printf '%s' "$out" | grep -qF "$VALUE"
}

# ---------------------------------------------------------------- create
reset
STUB_VERSIONS="" run create prod
check "create: exits 0" test "$code" -eq 0
check "create: adds exactly one version" test "$(grep -c 'versions add' "$log/gcloud.argv")" -eq 1
check "create: on the right secret and project" grep -q 'versions add vizualisation-origin-secret-prod --project palicanon-app --data-file=-' "$log/gcloud.argv"
check "create: the value is 64 hex characters on stdin" bash -c "grep -Eqx '[0-9a-f]{64}' '$log/gcloud.add.stdin'"
generated=$(tr -d '\n' <"$log/gcloud.add.stdin")
check "create: the generated value is on no argv" bash -c "! grep -qF '$generated' '$log/gcloud.argv'"
check "create: the generated value is not on screen" bash -c "! printf '%s' '$out' | grep -qF '$generated'"
check "create: no network call" no_curl

reset
STUB_VERSIONS="1" run create prod
check "create: does nothing when a version exists (exit 0)" test "$code" -eq 0
check "create: no version added when one exists" test "$(grep -c 'versions add' "$log/gcloud.argv")" -eq 0

# ---------------------------------------------------------------- check
reset
STUB_VERSIONS=1 run check prod
check "check: OK when the secret has an enabled version" test "$code" -eq 0
check "check: prints OK" out_is_ok
check "check: prints no value" no_value_anywhere
check "check: asks Secret Manager for the prod secret's versions" grep -q 'versions list vizualisation-origin-secret-prod --project palicanon-app' "$log/gcloud.argv"
check "check: no network call (no Worker, no host)" no_curl

reset
STUB_VERSIONS="" run check prod
check "check: not OK without a version" refused "create"
check "check: no network call without a version" no_curl

# ---------------------------------------------------------------- usage
# dev is gone (task 022), and `push` went with the Worker: both are refused.
reset
run create dev
check "usage: dev is refused" test "$code" -eq 2
check "usage: dev reaches no gcloud call" bash -c "! test -s '$log/gcloud.argv'"
reset
run push prod
check "usage: push is refused (no Worker)" test "$code" -eq 2
check "usage: push reaches no gcloud call and no curl call" bash -c "! test -s '$log/gcloud.argv' && ! test -s '$log/curl.argv'"
run create staging
check "usage: an unknown environment is refused" test "$code" -eq 2
run frobnicate prod
check "usage: an unknown command is refused" test "$code" -eq 2
run
check "usage: no argument is refused" test "$code" -eq 2
run create
check "usage: no environment is refused" test "$code" -eq 2

# ---------------------------------------------------------------- the source
# No Worker, no Cloudflare, no dev left in the script's code (comments may
# say what went).
code_lines=$(grep -vE '^[[:space:]]*#' "$script" | sed 's#/dev/null##g')
check "source: no Cloudflare or Worker code left" bash -c '! grep -qiE "cloudflare|worker|curl|ORIGIN_AUTH" <<<"$1"' _ "$code_lines"
check "source: no dev host or env left" bash -c '! grep -qE "map\.dev|\bdev\b" <<<"$1"' _ "$code_lines"

echo
echo "origin-secret.test.sh: $pass passed, $fail failed"
test "$fail" -eq 0
