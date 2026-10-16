#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
cargo build --release
mkdir -p bin
# A new inode, swapped in atomically: overwriting the file in place leaves the kernel's cached code
# signature stale, so macOS kills the binary on launch (and can kill a running panel).
cp target/release/cornercase-notch bin/.cornercase-notch.new
mv -f bin/.cornercase-notch.new bin/cornercase-notch
echo "built bin/cornercase-notch; link the plugin with: cornercase plugin link ."
