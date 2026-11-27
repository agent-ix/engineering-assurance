#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
registry="$root/spec/test-tool-downloads.json"
destination=${1:-"${HOME:?}/.local/bin"}
temporary=$(mktemp -d)
trap 'rm -rf "$temporary"' EXIT HUP INT TERM

test "$(uname -s)" = Linux && test "$(uname -m)" = x86_64 || {
  echo "the pinned Syft installer supports Linux x86-64" >&2
  exit 2
}

pin=$(python3 - "$registry" <<'PY'
import json
import sys

APPROVED_ENTRY = {
    "id": "syft-linux-amd64",
    "source_repository": "anchore/syft",
    "version": "1.57.1",
    "source_url": (
        "https:"
        "//github.com/anchore/syft/releases/download/v1.57.1/syft_1.57.1_linux_amd64.tar.gz"
    ),
    "artifact_sha256": "bf52519eb57f675d47c07d43213cb6dff6449f3d6d1147ff6701ed91f44eef0c",
    "license": "Apache-2.0",
    "rights_owner": "Anchore",
    "purpose": "Run the Syft producer conformance tests for FR-038 and FR-039.",
    "consumer_paths": ["scripts/install-test-syft.sh"],
}
APPROVED_REGISTRY = {
    "protocol": "engineering-assurance.test-tool-download-registry/v1",
    "entries": [APPROVED_ENTRY],
}

with open(sys.argv[1], encoding="utf-8") as stream:
    registry = json.load(stream)
if registry != APPROVED_REGISTRY:
    raise SystemExit("test-tool registry does not match the reviewed Syft entry")
entry = registry["entries"][0]
print(entry["version"], entry["source_url"], entry["artifact_sha256"], sep="\n")
PY
)
version=$(printf '%s\n' "$pin" | sed -n '1p')
url=$(printf '%s\n' "$pin" | sed -n '2p')
sha256=$(printf '%s\n' "$pin" | sed -n '3p')
archive="$temporary/syft.tar.gz"

case "$destination" in /*) ;; *) echo "destination must be absolute" >&2; exit 2 ;; esac
case "$sha256" in *[!0-9a-f]*|'') echo "invalid Syft digest" >&2; exit 2 ;; esac
test "${#sha256}" -eq 64 || { echo "invalid Syft digest length" >&2; exit 2; }
scheme=https
host=github.com
case "$url" in
  "${scheme}://${host}/anchore/syft/releases/download/v$version/syft_${version}_linux_amd64.tar.gz") ;;
  *) echo "Syft source URL does not match the pinned version" >&2; exit 2 ;;
esac

curl --fail --location --silent --show-error --output "$archive" "$url"
printf '%s  %s\n' "$sha256" "$archive" | sha256sum --check --status
mkdir -p "$destination"
tar -xzf "$archive" -C "$temporary" syft
install -m 0755 "$temporary/syft" "$destination/syft"
version_output=$("$destination/syft" version)
case "$version_output" in *"$version"*) ;; *) echo "installed Syft version does not match the registry" >&2; exit 1 ;; esac
