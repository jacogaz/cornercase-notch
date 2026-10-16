#!/bin/sh
# Set the manifest's version and rewrite its [download.*] tables so that they name the archives
# of that release: manifest.sh <manifest> <version> <folder with the archives>
# Only the release commit does this; a pull request never touches either.
set -eu

. "$(dirname "$0")/lib.sh"

REPO="jacogaz/cornercase-notch"
TARGETS="aarch64-apple-darwin x86_64-apple-darwin"

manifest="${1:?usage: manifest.sh <manifest> <version> <folder>}"
version="${2:?usage: manifest.sh <manifest> <version> <folder>}"
folder="${3:?usage: manifest.sh <manifest> <version> <folder>}"

work=$(mktemp "${TMPDIR:-/tmp}/manifest.XXXXXX")
trap 'rm -f "$work"' EXIT

# Everything before the first [download. line, with the first `version = ` line replaced and
# without the blank lines at its end.
awk -v version="$version" '/^\[download\./ { exit } !done && /^version = / { print "version = \"" version "\""; done = 1; next } { print }' "$manifest" |
    sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' >"$work"

for target in $TARGETS; do
    archive="cornercase-notch-$target.tar.gz"
    [ -f "$folder/$archive" ] || {
        echo "missing $folder/$archive" >&2
        exit 1
    }
    {
        printf '\n[download.%s]\n' "$target"
        printf 'url = "https://github.com/%s/releases/download/v%s/%s"\n' "$REPO" "$version" "$archive"
        printf 'sha256 = "%s"\n' "$(sha256 "$folder/$archive")"
    } >>"$work"
done

cat "$work" >"$manifest"
