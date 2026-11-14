#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX
set -euo pipefail

apply=false
repo=
while (($#)); do
  case "$1" in
    --repo)
      [[ $# -ge 2 ]] || { echo "--repo requires OWNER/REPO" >&2; exit 2; }
      repo=$2
      shift 2
      ;;
    --apply)
      apply=true
      shift
      ;;
    *)
      echo "usage: install-review-gate-ruleset.sh --repo agent-ix/ea-109-sandbox-NAME [--apply]" >&2
      exit 2
      ;;
  esac
done

if [[ ! "$repo" =~ ^agent-ix/ea-109-sandbox-[a-z0-9-]+$ ]]; then
  echo "ruleset installer is restricted to private EA-109 sandbox repositories" >&2
  exit 2
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
payload=$script_dir/../examples/main-review-gate-ruleset.json
if [[ ! -f "$payload" ]]; then
  echo "ruleset payload is missing" >&2
  exit 2
fi

private=$(gh api "repos/$repo" --jq '.private')
if [[ "$private" != true ]]; then
  echo "ruleset target must be private" >&2
  exit 2
fi

enabled=$(gh api "repos/$repo/actions/variables/EA_REVIEW_GATE_ENABLED" --jq '.value' 2>/dev/null || true)
if [[ "$enabled" != true ]]; then
  echo "set repository variable EA_REVIEW_GATE_ENABLED=true before applying" >&2
  exit 2
fi

existing=$(gh api --paginate "repos/$repo/rulesets" \
  | jq -sr '[.[][] | select(.name == "main-review-gate") | .id] | first // empty')
if [[ "$apply" != true ]]; then
  echo "validated private sandbox target: $repo"
  echo "ruleset payload: $payload"
  if [[ -n "$existing" ]]; then
    echo "would update ruleset id $existing"
  else
    echo "would create ruleset main-review-gate"
  fi
  cat "$payload"
  exit 0
fi

if [[ -n "$existing" ]]; then
  gh api --method PUT "repos/$repo/rulesets/$existing" --input "$payload"
else
  gh api --method POST "repos/$repo/rulesets" --input "$payload"
fi
