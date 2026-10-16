#!/bin/sh
# Pack the binary as bin/cornercase-notch in a gzipped tar that is the same every time:
# package.sh <target> <binary> <folder for the archive>
set -eu

target="${1:?usage: package.sh <target> <binary> <folder>}"
binary="${2:?usage: package.sh <target> <binary> <folder>}"
folder="${3:?usage: package.sh <target> <binary> <folder>}"

stage=$(mktemp -d "${TMPDIR:-/tmp}/package.XXXXXX")
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/bin" "$folder"
install -m 755 "$binary" "$stage/bin/cornercase-notch"
touch -t 200001010000 "$stage/bin" "$stage/bin/cornercase-notch"
COPYFILE_DISABLE=1 tar -C "$stage" --uid 0 --gid 0 --numeric-owner -cf - bin | gzip -n -9 >"$folder/cornercase-notch-$target.tar.gz"
