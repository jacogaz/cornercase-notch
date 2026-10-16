#!/bin/sh
set -eu

cd "$(dirname "$0")/.."
cargo build --release
mkdir -p bin
cp target/release/cornercase-notch bin/cornercase-notch
echo "built bin/cornercase-notch; link the plugin with: cornercase plugin link ."
