#!/usr/bin/env bash
# From search-front/scripts/serving-revision.sh, unchanged.
# The revision that serves the traffic, read from `gcloud run services
# describe --format json` on the standard input:
#
#   gcloud run services describe "$SERVICE" ... --format json | scripts/serving-revision.sh
#
# The entry of `status.traffic` at 100% names its revision (`revisionName`)
# or, when it follows the latest (`latestRevision: true`), has none: then the
# revision is `status.latestReadyRevisionName`. Prints the name. Fails (exit
# 1, nothing on the standard output) when the input is not that JSON, no
# entry is at 100%, or no revision can be named: deploy.yml records this name
# to roll back to, and an empty one would silently disable the rollback.
set -euo pipefail

serving=$(jq -r '
  ([.status.traffic[]? | select(.percent == 100)] | first) as $entry
  | if $entry == null then empty
    elif ($entry.revisionName // "") != "" then $entry.revisionName
    else (.status.latestReadyRevisionName // "") end
') || { echo "serving-revision: the input is not the JSON of a Cloud Run service" >&2; exit 1; }

if [ -z "$serving" ]; then
  echo "serving-revision: no revision serves 100% of the traffic (no entry at 100%, or latestRevision with no latestReadyRevisionName)" >&2
  exit 1
fi
printf '%s\n' "$serving"
