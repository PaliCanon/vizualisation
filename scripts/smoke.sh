#!/usr/bin/env bash
# The service answers, and only to the right caller: a smoke test of the
# vizualisation binary, run as the last step of the image build (there is no
# curl in the image: plain bash and /dev/tcp) and by the tests.
#
# From search-front/scripts/smoke.sh. What differs (design v4 of task 005):
# this service has a password gate (`VIZ_GATE`) and `/healthz`, and holds no
# API token, so there is no `token-pattern` and no token check (search-front
# looked for its API token's shape in the responses; nothing here could put
# one there). In its place, `leak`: no response echoes the throwaway origin
# secret or dev password. The boot check is new: an unset `VIZ_GATE` must
# stop the binary with exit 2 before it listens.
#
#   scripts/smoke.sh BINARY
#       starts BINARY on a free local port with VIZ_GATE=password and a
#       throwaway ORIGIN_SECRET and DEV_PASSWORD of its own (valid for this
#       run only, never the real ones), checks it, stops it; then starts it
#       once more without VIZ_GATE and checks it refuses to boot.
#   ORIGIN_SECRET=… DEV_PASSWORD=… scripts/smoke.sh --port PORT
#       checks a server already listening on 127.0.0.1:PORT, with the secret
#       and the password from the environment (never arguments).
#
# Checks, each named in the output: `listen` (the binary stays up and
# listens), `healthz` (/healthz with the origin header is 200 and says ok),
# `guard` (without the header, or with a wrong one, / is 403, and /healthz
# without it is 403), `gate` (/ and the .wasm without the password are 401,
# the 401 asks for it with WWW-Authenticate, a wrong password is 401), `root`
# (/ with both is 200 and names the page), `wasm` (the .wasm the page links is
# served as application/wasm), `leak` (no response holds the secret or the
# password), `boot` (BINARY mode: VIZ_GATE unset → exit 2, never listening).
# Exit 0 all good, 1 a check failed (named), 2 usage. Only status codes are
# printed, never a secret.
# SMOKE_WAIT: seconds to wait for the binary to listen, or to exit (default 30).
set -uo pipefail
export LC_ALL=C

work="$(mktemp -d)"
pid=""
cleanup() {
  if [ -n "$pid" ]; then kill "$pid" 2>/dev/null; wait "$pid" 2>/dev/null; fi
  rm -rf "$work"
}
trap cleanup EXIT

fail() { # check message
  echo "smoke: FAIL $1: $2" >&2
  exit 1
}

# get PORT PATH OUT [SECRET] [BASIC]: the raw HTTP/1.0 response in OUT.
get() {
  (
    exec 3<>"/dev/tcp/127.0.0.1/$1" || exit 1
    {
      printf 'GET %s HTTP/1.0\r\nHost: localhost\r\n' "$2"
      if [ -n "${4:-}" ]; then printf 'x-origin-auth: %s\r\n' "$4"; fi
      if [ -n "${5:-}" ]; then printf 'authorization: Basic %s\r\n' "$5"; fi
      printf '\r\n'
    } >&3
    cat <&3 >"$3"
  ) 2>/dev/null
}

status_of() { head -n 1 "$1" | cut -d' ' -f2; }
headers_of() { tr -d '\r' <"$1" | sed -n '1,/^$/p'; }

# The HTTP Basic credentials for PASSWORD (any user name: the gate reads the
# password only).
basic_of() { printf 'smoke:%s' "$1" | base64 | tr -d '\n'; }

# wait_listening PORT: waits until connected, or the binary is gone.
wait_listening() {
  local deadline=$((SECONDS + ${SMOKE_WAIT:-30}))
  until (exec 3<>"/dev/tcp/127.0.0.1/$1") 2>/dev/null; do
    if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
      fail listen "the binary exited before it listened"
    fi
    if [ "$SECONDS" -ge "$deadline" ]; then
      fail listen "nothing listens on port $1 after ${SMOKE_WAIT:-30}s"
    fi
    sleep 0.1
  done
}

# expect_status CHECK FILE WANT WHAT: FILE's status is WANT, or CHECK fails.
expect_status() {
  local status
  status="$(status_of "$2")"
  [ "$status" = "$3" ] || fail "$1" "$4 answered ${status:-nothing} ($3 wanted)"
}

run_checks() { # port secret password
  local port=$1 secret=$2 password=$3 basic wrong
  basic="$(basic_of "$password")"
  wrong="$(basic_of "not-the-password")"

  get "$port" /healthz "$work/healthz" "$secret" || fail healthz "no answer on port $port"
  expect_status healthz "$work/healthz" 200 "/healthz with the header"
  tr -d '\r' <"$work/healthz" | sed '1,/^$/d' | grep -aq '^ok' || fail healthz "/healthz does not say ok"
  echo "smoke: healthz ok"

  get "$port" / "$work/open" "" "$basic" || fail guard "no answer without the header"
  expect_status guard "$work/open" 403 "/ without the header"
  get "$port" / "$work/wrong" "not-the-secret" "$basic" || fail guard "no answer with a wrong header"
  expect_status guard "$work/wrong" 403 "/ with a wrong header"
  get "$port" /healthz "$work/healthz-open" || fail guard "no answer for /healthz without the header"
  expect_status guard "$work/healthz-open" 403 "/healthz without the header"
  echo "smoke: guard ok"

  get "$port" / "$work/nopass" "$secret" || fail gate "no answer without the password"
  expect_status gate "$work/nopass" 401 "/ without the password"
  headers_of "$work/nopass" | grep -iq '^www-authenticate:[[:space:]]*basic' \
    || fail gate "the 401 asks for no password (no WWW-Authenticate: Basic)"
  get "$port" / "$work/badpass" "$secret" "$wrong" || fail gate "no answer with a wrong password"
  expect_status gate "$work/badpass" 401 "/ with a wrong password"

  get "$port" / "$work/root" "$secret" "$basic" || fail root "no answer with the header and the password"
  expect_status root "$work/root" 200 "/ with the header and the password"
  grep -aEq '<title>[^<]*Suttas Map' "$work/root" || fail root "/ does not name the page"
  echo "smoke: root ok"

  local wasm
  wasm="$(grep -aEo '/pkg/[^"'"'"' <>]*\.wasm' "$work/root" | head -n 1)"
  [ -n "$wasm" ] || fail wasm "the page links no .wasm"
  get "$port" "$wasm" "$work/wasm" "$secret" "$basic" || fail wasm "no answer for $wasm"
  expect_status wasm "$work/wasm" 200 "$wasm"
  headers_of "$work/wasm" | grep -iq '^content-type:.*wasm' \
    || fail wasm "$wasm is not served as application/wasm"
  echo "smoke: wasm ok"

  get "$port" "$wasm" "$work/wasm-nopass" "$secret" || fail gate "no answer for $wasm without the password"
  expect_status gate "$work/wasm-nopass" 401 "$wasm without the password"
  echo "smoke: gate ok"

  local f
  for f in "$work"/*; do
    case "$f" in */server.log) continue ;; esac
    if grep -aqF -- "$secret" "$f" || grep -aqF -- "$password" "$f" || grep -aqF -- "$basic" "$f"; then
      fail leak "a response echoes the origin secret or the password ($(basename "$f"))"
    fi
  done
  echo "smoke: leak ok"
}

random_port() {
  local port
  # Two smoke tests started in the same second would share bash's seed.
  RANDOM=$(od -An -N2 -tu2 /dev/urandom | tr -d ' ')
  while :; do
    port=$((20000 + RANDOM % 40000))
    (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null || { echo "$port"; return; }
  done
}

# check_boot BINARY SECRET PASSWORD: without VIZ_GATE the binary must exit 2
# before it listens (design v4: the gate is never on or off by default).
check_boot() {
  local binary=$1 port code deadline
  port="$(random_port)"
  env -u VIZ_GATE ORIGIN_SECRET="$2" DEV_PASSWORD="$3" LEPTOS_SITE_ADDR="127.0.0.1:$port" \
    "$binary" >"$work/boot.log" 2>&1 &
  pid=$!
  deadline=$((SECONDS + ${SMOKE_WAIT:-30}))
  while kill -0 "$pid" 2>/dev/null; do
    if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null; then
      fail boot "the binary listens without VIZ_GATE (it must refuse to start)"
    fi
    if [ "$SECONDS" -ge "$deadline" ]; then
      fail boot "the binary still runs ${SMOKE_WAIT:-30}s after a boot without VIZ_GATE"
    fi
    sleep 0.1
  done
  wait "$pid"
  code=$?
  pid=""
  [ "$code" = 2 ] || fail boot "a boot without VIZ_GATE exited $code (2 wanted)"
  echo "smoke: boot ok"
}

case "${1:-}" in
  --port)
    [ -n "${2:-}" ] || { echo "usage: smoke.sh --port PORT (ORIGIN_SECRET and DEV_PASSWORD in the environment)" >&2; exit 2; }
    [ -n "${ORIGIN_SECRET:-}" ] || { echo "smoke: ORIGIN_SECRET is not set" >&2; exit 2; }
    [ -n "${DEV_PASSWORD:-}" ] || { echo "smoke: DEV_PASSWORD is not set" >&2; exit 2; }
    wait_listening "$2"
    run_checks "$2" "$ORIGIN_SECRET" "$DEV_PASSWORD"
    ;;
  "" | -*)
    echo "usage: smoke.sh BINARY | smoke.sh --port PORT" >&2
    exit 2
    ;;
  *)
    binary="$1"
    secret="smoke-$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
    password="smoke-$(od -An -N16 -tx1 /dev/urandom | tr -d ' \n')"
    port="$(random_port)"
    VIZ_GATE=password ORIGIN_SECRET="$secret" DEV_PASSWORD="$password" \
      LEPTOS_SITE_ADDR="127.0.0.1:$port" "$binary" >"$work/server.log" 2>&1 &
    pid=$!
    wait_listening "$port"
    # Something else may have taken the port first: our binary must still be up.
    sleep 0.3
    kill -0 "$pid" 2>/dev/null || fail listen "the binary exited (its port was taken?)"
    run_checks "$port" "$secret" "$password"
    kill "$pid" 2>/dev/null
    wait "$pid" 2>/dev/null
    pid=""
    check_boot "$binary" "$secret" "$password"
    ;;
esac
