#!/usr/bin/env bash
# The deployed service is right, whoever asks: run by deploy.yml on the
# tagged revision before it takes traffic, and on the public host after.
#
# From search-front/scripts/rollout-check.sh. What differs (design v4 of
# task 005): the checks of this service — `/healthz` behind the origin guard
# only, the password gate of dev (`--password-file`), prod's absence of any
# password prompt — in place of search-front's live search; no token-pattern
# and no token check (this service holds no API token: nothing could put one
# in a response). The exit codes, the secret's handling, the Cloudflare
# challenge and the Worker's 500 are search-front's.
#
#   scripts/rollout-check.sh [--password-file FILE] URL [HEADER_FILE]
#       the service itself (a revision URL, with the origin guard on). The
#       secret comes from HEADER_FILE (one line, `x-origin-auth: <secret>`,
#       passed to curl as `-H @file`) or from the ORIGIN_SECRET environment
#       variable (given to curl on its standard input): never on a command
#       line, never printed.
#   scripts/rollout-check.sh --via-worker [--password-file FILE] URL
#       the public host: the Worker adds the header, this script sends none
#       (and refuses to be given the secret).
#   scripts/rollout-check.sh --refused URL
#       a URL that must refuse everyone (the bare *.run.app): a 403 with an
#       empty body, without the header and with a wrong one. Takes no
#       password either.
#
# `--password-file FILE`: dev. FILE holds one line, `authorization: Basic
# <base64 of user:password>` (passed to curl as `-H @file`); the gate is
# checked: without it / is 401 and asks for it (WWW-Authenticate), with it
# the page and the files answer. Without the option: prod, no gate: / is 200
# with no password and nothing answers with WWW-Authenticate. So a dev
# checked without the file fails (its 401), and a prod that asks for a
# password fails.
#
# Checks, each named in the output: `healthz` (/healthz is 200 and says ok),
# `gate` (as above), `root` (/ is 200 and names the page), `wasm` (the .wasm
# the page links is served), `guard` (direct mode: without the header, or
# with a wrong one, / is 403; /healthz without it is 403), `refused`.
# Exit 0 all good, 1 a check failed (named), 2 usage, 3 Cloudflare answered
# with a challenge (cf-mitigated): the runner is not allowed, which says
# nothing about the service. 4 (--via-worker only) the Worker itself answered
# 500 — "The origin secret is not configured.", "The origin is not
# configured." or "The public origin is not configured." (the shared suffix
# `is not configured.`, whitespace and CR trimmed; infra/worker/proxy.js):
# the service was not reached and the remedy is `infra/scripts/origin-secret
# push`, not a rollback. Any other 500 is the service's, relayed, and is a
# failed check (1).
set -uo pipefail
export LC_ALL=C

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

usage() {
  echo "usage: rollout-check.sh [--via-worker | --refused] [--password-file FILE] URL [HEADER_FILE]" >&2
  exit 2
}

mode=direct
password_file=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --via-worker) mode=worker; shift ;;
    --refused) mode=refused; shift ;;
    --password-file)
      [ "$#" -ge 2 ] || usage
      password_file=$2
      [ -n "$password_file" ] || usage
      shift 2
      ;;
    --*) usage ;;
    *) break ;;
  esac
done
url="${1:-}"
header_file="${2:-}"
[ -n "$url" ] || usage
[ "$#" -le 2 ] || usage
url="${url%/}"

# The password: a file of one Basic line, never in refused mode.
if [ -n "$password_file" ]; then
  [ "$mode" != refused ] || { echo "rollout-check: --refused sends nothing of ours: give no password" >&2; exit 2; }
  [ -f "$password_file" ] || { echo "rollout-check: the password file is not a file" >&2; exit 2; }
  if [ "$(grep -c '' "$password_file")" != 1 ] \
    || ! grep -Eq '^[Aa]uthorization:[[:space:]]*Basic[[:space:]]+[A-Za-z0-9+/]+=*[[:space:]]*$' "$password_file"; then
    echo "rollout-check: the password file must hold one line, authorization: Basic <base64>" >&2
    exit 2
  fi
fi

# The secret: a file or the environment, direct mode only.
secret_mode=none
if [ "$mode" = direct ]; then
  if [ -n "$header_file" ]; then
    [ -f "$header_file" ] || { echo "rollout-check: the header file is not a file" >&2; exit 2; }
    if [ "$(grep -c '' "$header_file")" != 1 ] \
      || ! grep -Eq '^[Xx]-[Oo]rigin-[Aa]uth:[[:space:]]*[^[:space:]]+[[:space:]]*$' "$header_file"; then
      echo "rollout-check: the header file must hold one line, x-origin-auth: <secret>" >&2
      exit 2
    fi
    secret_mode=file
  elif [ -n "${ORIGIN_SECRET:-}" ]; then
    case "$ORIGIN_SECRET" in
      *[!A-Za-z0-9._~+/=-]*)
        echo "rollout-check: ORIGIN_SECRET has characters outside [A-Za-z0-9._~+/=-]" >&2
        exit 2
        ;;
    esac
    secret_mode=env
  else
    echo "rollout-check: no secret: give a HEADER_FILE or set ORIGIN_SECRET" >&2
    exit 2
  fi
elif [ -n "$header_file" ] || [ -n "${ORIGIN_SECRET:-}" ]; then
  echo "rollout-check: this mode never sends the origin secret: give none" >&2
  exit 2
fi

fail() { # check message
  echo "rollout-check: FAIL $1: $2" >&2
  exit 1
}

# fetch PATH NAME [auth|anon|wrong] [pass|nopass]: body in $work/NAME.body,
# headers in $work/NAME.head; prints the status ("000" when nothing
# answered). `pass` sends the password file, when there is one.
fetch() {
  local path=$1 name=$2 who=${3:-anon} pass=${4:-pass}
  local args=(-sS --max-time 60 -o "$work/$name.body" -D "$work/$name.head" -w '%{http_code}')
  if [ "$pass" = pass ] && [ -n "$password_file" ]; then
    args+=(-H "@$password_file")
  fi
  case "$who" in
    wrong) args+=(-H "x-origin-auth: not-the-secret") ;;
    auth)
      if [ "$secret_mode" = file ]; then
        args+=(-H "@$header_file")
      elif [ "$secret_mode" = env ]; then
        printf 'header = "x-origin-auth: %s"\n' "$ORIGIN_SECRET" \
          | curl "${args[@]}" --config - "$url$path" 2>/dev/null || printf '000'
        return
      fi
      ;;
  esac
  curl "${args[@]}" "$url$path" 2>/dev/null || printf '000'
}

# challenged NAME: exit 3 when Cloudflare's challenge, not the service, answered.
challenged() {
  if grep -iq '^cf-mitigated:' "$work/$1.head" 2>/dev/null; then
    echo "rollout-check: challenge: Cloudflare challenged this runner (cf-mitigated); the service was not reached" >&2
    exit 3
  fi
}

# workers_500 NAME STATUS: exit 4 when the Worker's own 500 (not the
# service's) answered through the public host.
workers_500() {
  [ "$mode" = worker ] && [ "$2" = 500 ] || return 0
  # Every Worker 500 body ends with `is not configured.` (proxy.js:
  # misconfigured()): matched on that suffix of the first 512 bytes, CR and
  # NUL dropped, leading and trailing whitespace trimmed, so a CRLF, a
  # trailing newline or padding added on the way does not hide it.
  local body
  body="$(head -c 512 "$work/$1.body" 2>/dev/null | tr -d '\r\0')"
  body="${body#"${body%%[![:space:]]*}"}"
  body="${body%"${body##*[![:space:]]}"}"
  if [ "${body%is not configured.}" != "$body" ]; then
    echo "rollout-check: worker-500: the Worker answered 500 (no origin secret, origin or public origin: run infra/scripts/origin-secret push); the service was not reached" >&2
    exit 4
  fi
}

expect() { # check name label status want
  if [ "$4" != "$5" ]; then
    challenged "$2"
    workers_500 "$2" "$4"
    fail "$1" "$3 answered $4 ($5 wanted)"
  fi
}

asks_password() { grep -iq '^www-authenticate:' "$work/$1.head" 2>/dev/null; }

who=auth
[ "$mode" = worker ] && who=anon

if [ "$mode" = refused ]; then
  for probe in anon wrong; do
    for path in / /healthz; do
      status="$(fetch "$path" "r-$probe" "$probe")"
      [ "$status" = 403 ] || { challenged "r-$probe"; fail refused "$path answered $status ($probe request; 403 wanted)"; }
      [ ! -s "$work/r-$probe.body" ] || fail refused "the 403 for $path has a body"
    done
  done
  echo "rollout-check: refused ok"
  exit 0
fi

status="$(fetch /healthz healthz "$who")"
expect healthz healthz /healthz "$status" 200
grep -aq '^ok' "$work/healthz.body" || fail healthz "/healthz does not say ok"
echo "rollout-check: healthz ok"

if [ -n "$password_file" ]; then
  status="$(fetch / nopass "$who" nopass)"
  expect gate nopass "/ without the password" "$status" 401
  asks_password nopass || fail gate "the 401 asks for no password (no WWW-Authenticate)"
fi

status="$(fetch / root "$who")"
expect root root / "$status" 200
grep -aEq '<title>[^<]*Suttas Map' "$work/root.body" || fail root "/ does not name the page"
if [ -z "$password_file" ]; then
  for name in healthz root; do
    ! asks_password "$name" || fail gate "the $name response asks for a password (WWW-Authenticate): the gate is on"
  done
fi
echo "rollout-check: gate ok ($([ -n "$password_file" ] && echo password || echo off))"
echo "rollout-check: root ok"

wasm="$(grep -aEo '/pkg/[^"'"'"' <>]*\.wasm' "$work/root.body" | head -n 1)"
[ -n "$wasm" ] || fail wasm "the page links no .wasm"
status="$(fetch "$wasm" wasm "$who")"
expect wasm wasm "$wasm" "$status" 200
echo "rollout-check: wasm ok"

if [ "$mode" = direct ]; then
  status="$(fetch / open anon)"
  [ "$status" = 403 ] || { challenged open; fail guard "/ without the header answered $status (403 wanted)"; }
  status="$(fetch / wrongone wrong)"
  [ "$status" = 403 ] || fail guard "/ with a wrong header answered $status (403 wanted)"
  status="$(fetch /healthz healthz-open anon)"
  [ "$status" = 403 ] || fail guard "/healthz without the header answered $status (403 wanted)"
  echo "rollout-check: guard ok"
fi
