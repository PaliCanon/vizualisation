#!/usr/bin/env bash
# From search-front/scripts/serving-revision.test.sh, unchanged.
# Tests for scripts/serving-revision.sh (task 007, review round 2): the
# revision that serves the traffic, read from `gcloud run services describe
# --format json`, whether the 100% entry names it or says `latestRevision`.
# Run: bash scripts/serving-revision.test.sh
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
script="$here/serving-revision.sh"
pass=0
fail=0
expect() { # expect NAME WANT SERVICE_JSON   (WANT "<failed>": exit 1, nothing on stdout)
  local name=$1 want=$2 json=$3 got err
  got=$(printf '%s' "$json" | bash "$script" 2>/dev/null) && err=0 || err=$?
  [ "$err" = 0 ] || got="<failed>"
  [ "$err" = 0 ] || [ "$err" = 1 ] || got="<exit $err>"
  if [ "$got" = "$want" ]; then pass=$((pass + 1)); echo "ok   $name"; else fail=$((fail + 1)); echo "FAIL $name: want '$want', got '$got'"; fi
}

expect "a named revision at 100%" rev-00003 \
  '{"status":{"traffic":[{"revisionName":"rev-00003","percent":100}],"latestReadyRevisionName":"rev-00004"}}'
expect "latestRevision at 100% is the latest ready revision" rev-00004 \
  '{"status":{"traffic":[{"latestRevision":true,"percent":100}],"latestReadyRevisionName":"rev-00004"}}'
expect "an empty revisionName with latestRevision" rev-00004 \
  '{"status":{"traffic":[{"revisionName":"","latestRevision":true,"percent":100}],"latestReadyRevisionName":"rev-00004"}}'
expect "a named revision wins over the latest ready one" rev-00003 \
  '{"status":{"traffic":[{"revisionName":"rev-00003","latestRevision":true,"percent":100}],"latestReadyRevisionName":"rev-00004"}}'
expect "tagged entries at 0% are not the serving one" rev-00003 \
  '{"status":{"traffic":[{"revisionName":"rev-00005","percent":0,"tag":"sha-abc","url":"https://x"},{"revisionName":"rev-00003","percent":100}],"latestReadyRevisionName":"rev-00005"}}'
expect "latestRevision without a latest ready revision is a failure" "<failed>" \
  '{"status":{"traffic":[{"latestRevision":true,"percent":100}]}}'
expect "an empty latestReadyRevisionName is a failure" "<failed>" \
  '{"status":{"traffic":[{"latestRevision":true,"percent":100}],"latestReadyRevisionName":""}}'
expect "no traffic entry is a failure" "<failed>" '{"status":{"traffic":[]}}'
expect "no status is a failure" "<failed>" '{}'
expect "a split without a 100% entry is a failure" "<failed>" \
  '{"status":{"traffic":[{"revisionName":"a","percent":50},{"revisionName":"b","percent":50}]}}'
expect "something that is not JSON is a failure" "<failed>" 'ERROR: not found'
expect "nothing at all is a failure" "<failed>" ''

echo
echo "serving-revision.test.sh: $pass passed, $fail failed"
test "$fail" -eq 0
