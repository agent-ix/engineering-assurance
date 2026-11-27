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

with open(sys.argv[1], encoding="utf-8") as stream:
    registry = json.load(stream)
if registry.get("protocol") != "engineering-assurance.test-tool-download-registry/v1":
    raise SystemExit("unsupported test-tool registry")
entries = [entry for entry in registry.get("entries", []) if entry.get("id") == "syft-linux-amd64"]
if len(entries) != 1:
    raise SystemExit("Syft pin is missing or ambiguous")
entry = entries[0]
if entry.get("source_repository") != "anchore/syft" or entry.get("version") != "1.57.1":
    raise SystemExit("Syft pin does not match the supported release")
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
