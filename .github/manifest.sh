#!/bin/sh
# Set the manifest's version and rewrite its [download.*] tables so that they name the archives
# of that release: manifest.sh <manifest> <version> <folder with the archives>
# Only the release commit does this; a pull request never touches either.
set -eu

. "$(dirname "$0")/lib.sh"

manifest="${1:?usage: manifest.sh <manifest> <version> <folder>}"
version="${2:?usage: manifest.sh <manifest> <version> <folder>}"
folder="${3:?usage: manifest.sh <manifest> <version> <folder>}"

work=$(mktemp "${TMPDIR:-/tmp}/manifest.XXXXXX")
trap 'rm -f "$work"' EXIT

archive_sha() {
    [ -f "$folder/cornercase-notch-$1.tar.gz" ] || {
        echo "missing $folder/cornercase-notch-$1.tar.gz" >&2
        return 1
    }
    sha256 "$folder/cornercase-notch-$1.tar.gz"
}

render_manifest "$manifest" "$version" archive_sha >"$work"
cat "$work" >"$manifest"
