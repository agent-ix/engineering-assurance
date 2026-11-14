#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX
set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: prepare-review-gate-request.sh POLICY_REPO BASE_SHA REPOSITORY PULL_REQUEST" >&2
  exit 2
fi

policy_repo=$1
base_sha=$2
repository=$3
pull_request=$4

if [[ ! "$base_sha" =~ ^[0-9a-f]{40,64}$ || ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ || ! "$pull_request" =~ ^[1-9][0-9]*$ ]]; then
  echo "review-gate input reference is invalid" >&2
  exit 2
fi
if [[ ! "${REVIEW_GATE_HEAD_SHA:-}" =~ ^[0-9a-f]{40,64}$ ]]; then
  echo "review-gate head revision is invalid" >&2
  exit 2
fi

policy=$(git -C "$policy_repo" show "$base_sha:.engineering-assurance/approval-policy.json" 2>/dev/null || printf 'null')
if ! jq -e . >/dev/null <<<"$policy"; then
  policy=null
fi

reviews_file=$(mktemp)
types_file=$(mktemp)
edit_metadata_file=$(mktemp)
trap 'rm -f "$reviews_file" "$types_file" "$edit_metadata_file"' EXIT

source_unavailable=false
if gh api --paginate --slurp "repos/$repository/pulls/$pull_request/reviews" \
  | jq -c 'add // []' >"$reviews_file"; then
  :
else
  source_unavailable=true
  printf '[]\n' >"$reviews_file"
fi

# REST review bodies do not identify post-submission edits. Query the full
# GraphQL review connection and fail closed if its edit metadata is incomplete.
if [[ "$source_unavailable" == false ]]; then
  owner=${repository%%/*}
  repo_name=${repository#*/}
  if ! gh api graphql --paginate --slurp \
    -F owner="$owner" -F name="$repo_name" -F number="$pull_request" \
    -f query='query($owner:String!, $name:String!, $number:Int!, $endCursor:String) {
      repository(owner:$owner, name:$name) {
        pullRequest(number:$number) {
          reviews(first:100, after:$endCursor) {
            totalCount
            pageInfo { hasNextPage }
            nodes {
              fullDatabaseId
              lastEditedAt
              userContentEdits(first:1) { totalCount pageInfo { hasNextPage } }
            }
          }
        }
      }
    }' >"$edit_metadata_file"; then
    source_unavailable=true
  elif ! jq -e '
    length > 0 and
    (.[0].data.repository.pullRequest.reviews.totalCount as $expected |
      ([.[].data.repository.pullRequest.reviews.totalCount] | all(. == $expected)) and
      ([.[].data.repository.pullRequest.reviews.nodes[]] | length == $expected) and
      ([.[].data.repository.pullRequest.reviews.nodes[] |
        select(.fullDatabaseId != null and
          (.userContentEdits | type == "object") and
          (.userContentEdits.totalCount | type == "number") and
          (.userContentEdits.pageInfo.hasNextPage | type == "boolean") and
          (.userContentEdits.pageInfo.hasNextPage == (.userContentEdits.totalCount > 1)))] | length == $expected) and
    (.[-1].data.repository.pullRequest.reviews.pageInfo.hasNextPage == false)
  ' "$edit_metadata_file" >/dev/null 2>&1; then
    source_unavailable=true
  elif ! diff -q \
    <(jq -r '.[].id | tostring' "$reviews_file" | sort -u) \
    <(jq -r '[.[].data.repository.pullRequest.reviews.nodes[].fullDatabaseId | tostring] | .[]' "$edit_metadata_file" | sort -u) \
    >/dev/null; then
    source_unavailable=true
  fi
fi

account_types='[]'
if [[ "$policy" != null ]]; then
  while IFS= read -r account; do
    source=$(jq -r '.source' <<<"$account")
    id=$(jq -r '.id' <<<"$account")
    account_type=unavailable
    if [[ "$source" == github ]] && user=$(gh api "/user/$id" 2>/dev/null); then
      case "$(jq -r '.type // "unknown"' <<<"$user")" in
        User) account_type=human ;;
        Bot|App) account_type=bot_or_app ;;
        *) account_type=unknown ;;
      esac
    elif [[ "$source" != github ]]; then
      account_type=unknown
    fi
    jq -cn --arg source "$source" --arg id "$id" --arg account_type "$account_type" \
      '{account:{source:$source,id:$id},account_type:$account_type}' >>"$types_file"
  done < <(jq -c '([.individuals[] | .[]] + (.agents // [])) | unique_by([.source,.id])[]' <<<"$policy")
  account_types=$(jq -s '.' "$types_file")
fi

if [[ "$source_unavailable" == false ]]; then
  edited_ids=$(jq -r '[.[].data.repository.pullRequest.reviews.nodes[] |
    select(.lastEditedAt != null or .userContentEdits.totalCount > 0) |
    .fullDatabaseId | tostring] | unique | .[]' "$edit_metadata_file")
else
  edited_ids=
fi

reviews=$(jq -c --arg pr "$repository#$pull_request" --arg edited_ids "$edited_ids" '
  ($edited_ids | split("\n") | map(select(length > 0))) as $edited |
  [.[] as $review | {
    review_id: $review.id,
    account: {source:"github", id:($review.user.id | tostring)},
    account_type:
      (if $review.user.type == "User" then "human"
       elif $review.user.type == "Bot" then "bot_or_app"
       else "unknown" end),
    state: $review.state,
    submitted_body: ($review.body // ""),
    body_edited: ($edited | any(. == ($review.id | tostring))),
    submitted_at: $review.submitted_at,
    commit: $review.commit_id,
    pull_request: $pr
  }]
' "$reviews_file")

jq -cn \
  --arg protocol "engineering-assurance.review-gate-request/v1" \
  --arg current_head "$REVIEW_GATE_HEAD_SHA" \
  --argjson policy "$policy" \
  --argjson reviews "$reviews" \
  --argjson account_types "$account_types" \
  --argjson source_unavailable "$source_unavailable" \
  '{protocol:$protocol,current_head:$current_head,policy:$policy,reviews:$reviews,account_types:$account_types,source_unavailable:$source_unavailable}'
