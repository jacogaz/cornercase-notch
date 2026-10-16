#!/bin/sh
# Tries manifest.sh and version.sh on copies, with fake archives and a scratch repository.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/notch-test.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
ok() { echo "ok: $1"; }
bad() { echo "FAILED: $1" >&2; exit 1; }

# manifest.sh: rewrites the tables, twice in a row gives the same file, keeps the top.
cp "$here/cornercase-plugin.toml" "$tmp/manifest.toml"
mkdir "$tmp/dist"
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    printf 'fake %s' "$target" >"$tmp/dist/cornercase-notch-$target.tar.gz"
done
"$here/.github/manifest.sh" "$tmp/manifest.toml" 9.9.9 "$tmp/dist"
cp "$tmp/manifest.toml" "$tmp/first.toml"
"$here/.github/manifest.sh" "$tmp/manifest.toml" 9.9.9 "$tmp/dist"
cmp -s "$tmp/first.toml" "$tmp/manifest.toml" || bad "manifest.sh is not idempotent"
grep -q '^url = "https://github.com/jacogaz/cornercase-notch/releases/download/v9.9.9/cornercase-notch-aarch64-apple-darwin.tar.gz"$' "$tmp/manifest.toml" || bad "aarch64 url"
grep -q '^url = "https://github.com/jacogaz/cornercase-notch/releases/download/v9.9.9/cornercase-notch-x86_64-apple-darwin.tar.gz"$' "$tmp/manifest.toml" || bad "x86_64 url"
[ "$(grep -c '^sha256 = "[0-9a-f]\{64\}"$' "$tmp/manifest.toml")" = 2 ] || bad "sha256 lines"
[ "$(grep -c '^\[download\.' "$tmp/manifest.toml")" = 2 ] || bad "download tables"
head -n "$(awk '/^\[download\./ { print NR - 2; exit }' "$tmp/manifest.toml")" "$tmp/manifest.toml" | cmp -s - "$here/cornercase-plugin.toml" || bad "the top of the manifest changed"
ok "manifest.sh"

# version.sh: in a scratch repository.
repo="$tmp/repo"
mkdir -p "$repo/src" "$repo/.github"
cp "$here/.github/version.sh" "$repo/.github/"
cd "$repo"
git init -q -b main
git config user.name test
git config user.email test@example.com
printf '[package]\nversion = "0.1.0"\n' >Cargo.toml
printf 'version = "0.1.0"\n' >cornercase-plugin.toml
printf '## 0.1.0\n\n- first\n' >CHANGELOG.md
echo 'fn main() {}' >src/main.rs
touch Cargo.lock rust-toolchain.toml
git add -A && git commit -q -m first

sh .github/version.sh release | grep -q 'release=yes' || bad "unreleased version should release"
git tag v0.1.0
sh .github/version.sh release | grep -q 'release=no' || bad "released version should not release"

git switch -q -c change
echo '// change' >>src/main.rs
git commit -qam change
if sh .github/version.sh check main >/dev/null 2>&1; then bad "an app change without a new version passed"; fi
if sh .github/version.sh release >/dev/null 2>&1; then bad "a changed app under a released version passed"; fi
sed -i.bak 's/0.1.0/0.2.0/' Cargo.toml && rm Cargo.toml.bak
git commit -qam bump
if sh .github/version.sh check main >/dev/null 2>&1; then bad "a manifest behind Cargo.toml passed"; fi
sed -i.bak 's/0.1.0/0.2.0/' cornercase-plugin.toml && rm cornercase-plugin.toml.bak
git commit -qam manifest
if sh .github/version.sh check main >/dev/null 2>&1; then bad "a version without a changelog section passed"; fi
printf '## 0.2.0\n\n- second\n' | cat - CHANGELOG.md >CHANGELOG.new && mv CHANGELOG.new CHANGELOG.md
git commit -qam changelog
sh .github/version.sh check main >/dev/null || bad "a complete release pull request failed"
ok "version.sh"
