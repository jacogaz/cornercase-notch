#!/bin/sh
# Rewrite the [download.*] tables at the end of the plugin manifest so that they name
# the archives of a release: manifest.sh <manifest> <version> <folder with the archives>
set -eu

REPO="jacogaz/cornercase-notch"
TARGETS="aarch64-apple-darwin x86_64-apple-darwin"

manifest="${1:?usage: manifest.sh <manifest> <version> <folder>}"
version="${2:?usage: manifest.sh <manifest> <version> <folder>}"
folder="${3:?usage: manifest.sh <manifest> <version> <folder>}"

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    else
        shasum -a 256 "$1" | cut -d ' ' -f 1
    fi
}

work=$(mktemp "${TMPDIR:-/tmp}/manifest.XXXXXX")
trap 'rm -f "$work"' EXIT

# Everything before the first [download. line, without the blank lines at its end.
awk '/^\[download\./ { exit } { print }' "$manifest" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' >"$work"

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
