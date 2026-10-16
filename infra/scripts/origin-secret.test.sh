#!/usr/bin/env bash
# From search-front/infra/scripts/origin-secret.test.sh. What differs: the
# names (vizualisation-origin-secret-<env>, the Worker vizualisation-<env>),
# the hosts (map.dev.palicanon.app, map.palicanon.app), and `check` asks the
# host's /healthz: dev's / is behind the password gate (401), /healthz is
# behind the origin guard only, so it is what says the Worker's value works.
#
# Tests for infra/scripts/origin-secret (search-front 007, slice S4): `gcloud` and
# `curl` are stubs first on PATH that log their argv and stdin. The contract
# under test: the secret value travels on stdin only (never in an argument,
# never on screen), `create` is idempotent, `check` prints OK or why not.
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
CFTOKEN="cf-token-for-tests-fedcba9876543210"

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
# curl: logs argv; a `--config -` call logs its stdin config and answers by
# its method (PUT → the status code, GET → a body); a plain call is the
# public host check (→ the status code).
cat >"$bin/curl" <<STUB
#!/usr/bin/env bash
printf '%s\n' "\$*" >>"$log/curl.argv"
if [ "\${1:-}" = "--config" ] && [ "\${2:-}" = "-" ]; then
  cfg=\$(cat)
  printf '%s\n---\n' "\$cfg" >>"$log/curl.stdin"
  case "\$cfg" in
    *'request = "PUT"'*) printf '%s' "\${STUB_PUT_CODE:-200}" ;;
    *) printf '%s' "\${STUB_WORKER_SECRETS:-{\"result\":[{\"name\":\"ORIGIN_AUTH\",\"type\":\"secret_text\"}],\"success\":true}}" ;;
  esac
else
  printf '%s' "\${STUB_HOST_CODE:-200}"
fi
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
# run <args...> → $out (stdout+stderr), $code
run() {
  out=$(env PATH="$bin:$PATH" CLOUDFLARE_API_TOKEN="${TOKEN-$CFTOKEN}" "$script" "$@" 2>&1)
  code=$?
}
out_has() { printf '%s' "$out" | grep -q -- "$1"; }
out_is_ok() { [ "$out" = "OK" ]; }
refused() { [ "$code" -ne 0 ] && out_has "not OK" && { [ -z "${1:-}" ] || out_has "$1"; }; }
no_value_anywhere() {
  ! grep -qF "$VALUE" "$log/gcloud.argv" "$log/curl.argv" 2>/dev/null && ! printf '%s' "$out" | grep -qF "$VALUE"
}
no_cf_token_in_argv() { ! grep -qF "$CFTOKEN" "$log/gcloud.argv" "$log/curl.argv" 2>/dev/null && ! printf '%s' "$out" | grep -qF "$CFTOKEN"; }

# ---------------------------------------------------------------- create
reset
STUB_VERSIONS="" run create dev
check "create: exits 0" test "$code" -eq 0
check "create: adds exactly one version" test "$(grep -c 'versions add' "$log/gcloud.argv")" -eq 1
check "create: on the right secret and project" grep -q 'versions add vizualisation-origin-secret-dev --project palicanon-app --data-file=-' "$log/gcloud.argv"
check "create: the value is 64 hex characters on stdin" bash -c "grep -Eqx '[0-9a-f]{64}' '$log/gcloud.add.stdin'"
generated=$(tr -d '\n' <"$log/gcloud.add.stdin")
check "create: the generated value is on no argv" bash -c "! grep -qF '$generated' '$log/gcloud.argv'"
check "create: the generated value is not on screen" bash -c "! printf '%s' '$out' | grep -qF '$generated'"

reset
STUB_VERSIONS="1" run create dev
check "create: does nothing when a version exists (exit 0)" test "$code" -eq 0
check "create: no version added when one exists" test "$(grep -c 'versions add' "$log/gcloud.argv")" -eq 0

reset
STUB_VERSIONS="" run create prod
check "create prod: its own secret" grep -q 'versions add vizualisation-origin-secret-prod ' "$log/gcloud.argv"

# ---------------------------------------------------------------- push
reset
run push dev
check "push: exits 0 on HTTP 200" test "$code" -eq 0
check "push: reads the latest version" grep -q 'versions access latest --secret vizualisation-origin-secret-dev --project palicanon-app' "$log/gcloud.argv"
check "push: PUTs to the dev Worker's secrets" grep -q 'workers/scripts/vizualisation-dev/secrets' "$log/curl.stdin"
check "push: names the binding ORIGIN_AUTH" grep -q 'ORIGIN_AUTH' "$log/curl.stdin"
check "push: the value reaches curl on stdin" grep -qF "$VALUE" "$log/curl.stdin"
check "push: curl is called with --config - and nothing else" bash -c "test -s '$log/curl.argv' && test \$(grep -vc -- '^--config -\$' '$log/curl.argv') -eq 0"
check "push: the value is on no argv and not on screen" no_value_anywhere
check "push: the Cloudflare token is on no argv and not on screen" no_cf_token_in_argv

reset
# Cloudflare answers 201 Created when the secret is new on the Worker (the
# first push after an apply): a success, as 200 is (search-front's task 014).
STUB_PUT_CODE=201 run push dev
check "push: exits 0 on HTTP 201 (a new secret)" test "$code" -eq 0

STUB_PUT_CODE=403 run push dev
check "push: exits non-zero when Cloudflare refuses" test "$code" -ne 0
check "push: says the status, not the value" out_has 403
check "push: refusal output holds no value" no_value_anywhere

reset
TOKEN="" run push dev
check "push: no Cloudflare token → non-zero" test "$code" -ne 0
check "push: no Cloudflare token → no call at all" bash -c "! test -s '$log/curl.argv' && ! grep -q 'versions access' '$log/gcloud.argv'"
unset TOKEN

# ---------------------------------------------------------------- check
reset
STUB_VERSIONS=1 run check dev
check "check: OK when the version, the Worker's secret and the host agree" test "$code" -eq 0
check "check: prints OK" out_is_ok
check "check: prints no value" no_value_anywhere
check "check: asks the dev host's /healthz" grep -q 'https://map.dev.palicanon.app/healthz' "$log/curl.argv"

reset
STUB_VERSIONS="" run check dev
# `check` lists versions through gcloud: no version → not OK.
check "check: not OK without a version" refused
reset
STUB_VERSIONS="1" STUB_WORKER_SECRETS='{"result":[],"success":true}' run check dev
check "check: not OK when the Worker has no secret" refused
reset
STUB_VERSIONS="1" STUB_HOST_CODE=500 run check dev
check "check: 500 → not OK (the Worker lost its secret)" refused 500
reset
STUB_VERSIONS="1" STUB_HOST_CODE=403 run check dev
check "check: 403 → not OK (the service refuses the Worker's value)" refused 403
reset
STUB_VERSIONS="1" run check prod
check "check prod: asks the prod host's /healthz" grep -q 'https://map.palicanon.app/healthz' "$log/curl.argv"

# ---------------------------------------------------------------- usage
reset
run create staging
check "usage: an unknown environment is refused" test "$code" -eq 2
run frobnicate dev
check "usage: an unknown command is refused" test "$code" -eq 2
run
check "usage: no argument is refused" test "$code" -eq 2

# ---------------------------------------------------------------- the comment
# The rotation order is written in the script's header and in infra/README.md:
# the Worker's new value first (`push`), then a new revision. The header said
# the reverse once (review round 2): it is pinned here.
rotation=$(sed -n '/Rotation/,/^set /p' "$script" | tr -d '#\n' | tr -s ' ')
check "comment: the rotation paragraph is found" test -n "$rotation"
check "comment: the rotation pushes to the Worker before the new revision" \
  bash -c '[[ "$1" =~ push.*(new|start).*revision ]]' _ "$rotation"
check "comment: the rotation does not roll the revision before the push" \
  bash -c '! [[ "$1" =~ (roll|start).*revision.*then.*push ]]' _ "$rotation"

echo
echo "origin-secret.test.sh: $pass passed, $fail failed"
test "$fail" -eq 0
