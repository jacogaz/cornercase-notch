#!/bin/sh
# Tries the release scripts in scratch repositories, with fake archives, a bare repository as
# origin and a stub `gh` on the PATH. Nothing here touches this repository or GitHub.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/notch-test.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
ok() { echo "ok: $1"; }
bad() { echo "FAILED: $1" >&2; exit 1; }
fails() { if "$@" >"$tmp/out" 2>&1; then return 1; fi; }

ARCHIVES="cornercase-notch-aarch64-apple-darwin.tar.gz cornercase-notch-x86_64-apple-darwin.tar.gz"

# A stub gh: its state is in $stub (release = exists, assets = one name per line, calls = log).
stub="$tmp/stub"
mkdir -p "$stub" "$tmp/bin"
cat >"$tmp/bin/gh" <<STUB
#!/bin/sh
echo "\$*" >>"$stub/calls"
case "\$1 \$2" in
    "release view")
        [ -f "$stub/release" ] || exit 1
        case "\$*" in *--json*) cat "$stub/assets" ;; esac ;;
    "release create")
        touch "$stub/release"
        for arg in "\$@"; do case "\$arg" in *.tar.gz) basename "\$arg" >>"$stub/assets" ;; esac; done ;;
    "release upload") basename "\$4" >>"$stub/assets" ;;
    *) exit 1 ;;
esac
STUB
chmod +x "$tmp/bin/gh"
PATH="$tmp/bin:$PATH"
export PATH
reset_stub() { rm -f "$stub/release" "$stub/assets" "$stub/calls"; touch "$stub/assets" "$stub/calls"; }

# A scratch repository with an origin, one commit on main, and the release scripts.
scratch() {
    rm -rf "$tmp/origin.git" "$tmp/repo" "$tmp/dist"
    reset_stub
    git init -q --bare -b main "$tmp/origin.git"
    repo="$tmp/repo"
    mkdir -p "$repo/src" "$repo/.github"
    cd "$repo"
    git init -q -b main
    git config user.name test
    git config user.email test@example.com
    git remote add origin "$tmp/origin.git"
    for script in lib version manifest publish package; do cp "$here/.github/$script.sh" .github/; done
    printf '[package]\nversion = "0.1.0"\n' >Cargo.toml
    printf 'id = "x"\nversion = "0.1.0"\n\n[[window]]\nid = "panel"\n' >cornercase-plugin.toml
    printf '## 0.1.0\n\n- first\n' >CHANGELOG.md
    echo 'fn main() {}' >src/main.rs
    touch Cargo.lock rust-toolchain.toml
    git add -A && git commit -q -m first
    git push -q origin main
    mkdir "$tmp/dist"
    for archive in $ARCHIVES; do printf 'fake %s' "$archive" >"$tmp/dist/$archive"; done
}
version() { sh .github/version.sh "$@"; }
publish() { DIST="$tmp/dist" sh .github/publish.sh "$@"; }

# --- manifest.sh
cp "$here/cornercase-plugin.toml" "$tmp/manifest.toml"
mkdir "$tmp/fake"
for archive in $ARCHIVES; do printf 'fake %s' "$archive" >"$tmp/fake/$archive"; done
"$here/.github/manifest.sh" "$tmp/manifest.toml" 9.9.9 "$tmp/fake"
cp "$tmp/manifest.toml" "$tmp/first.toml"
"$here/.github/manifest.sh" "$tmp/manifest.toml" 9.9.9 "$tmp/fake"
cmp -s "$tmp/first.toml" "$tmp/manifest.toml" || bad "manifest.sh is not idempotent"
[ "$(sed -n 's/^version = "\(.*\)"$/\1/p' "$tmp/manifest.toml")" = 9.9.9 ] || bad "manifest.sh did not set the version"
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    grep -q "^url = \"https://github.com/jacogaz/cornercase-notch/releases/download/v9.9.9/cornercase-notch-$target.tar.gz\"\$" "$tmp/manifest.toml" || bad "$target url"
done
[ "$(grep -c '^sha256 = "[0-9a-f]\{64\}"$' "$tmp/manifest.toml")" = 2 ] || bad "sha256 lines"
[ "$(grep -c '^\[download\.' "$tmp/manifest.toml")" = 2 ] || bad "download tables"
sed 's/^version = ".*"$/version = "X"/' "$here/cornercase-plugin.toml" >"$tmp/a.toml"
head -n "$(awk '/^\[download\./ { print NR - 2; exit }' "$tmp/manifest.toml")" "$tmp/manifest.toml" | sed 's/^version = ".*"$/version = "X"/' | cmp -s - "$tmp/a.toml" || bad "the rest of the manifest changed"
ok "manifest.sh sets the version and the downloads"

# --- package.sh
printf 'binary' >"$tmp/binary"
"$here/.github/package.sh" aarch64-apple-darwin "$tmp/binary" "$tmp/p1"
touch -t 202501010000 "$tmp/binary"
"$here/.github/package.sh" aarch64-apple-darwin "$tmp/binary" "$tmp/p2"
cmp -s "$tmp/p1/cornercase-notch-aarch64-apple-darwin.tar.gz" "$tmp/p2/cornercase-notch-aarch64-apple-darwin.tar.gz" || bad "package.sh is not reproducible"
[ "$(tar -tzf "$tmp/p1/cornercase-notch-aarch64-apple-darwin.tar.gz" | sort | tr '\n' ' ')" = "bin/ bin/cornercase-notch " ] || bad "the archive holds more than bin/cornercase-notch"
ok "package.sh is reproducible and holds only bin/cornercase-notch"

# --- a new release end to end, then nothing to do
scratch
version release | grep -q 'release=new' || bad "an unreleased version should be new"
publish new >/dev/null 2>&1 || bad "publish new failed"
[ "$(git rev-parse origin/main 2>/dev/null || git -C "$tmp/origin.git" rev-parse main)" = "$(git rev-parse HEAD)" ] || bad "the release commit did not reach main"
git -C "$tmp/origin.git" rev-parse -q --verify refs/tags/v0.1.0 >/dev/null || bad "the tag was not pushed"
grep -q 'release create v0.1.0' "$stub/calls" || bad "no release was created"
grep -q '^url = ".*/v0.1.0/' cornercase-plugin.toml || bad "the manifest names no downloads"
version release | grep -q 'release=no' || bad "a whole release should be left alone"
ok "a new release is tagged, released and then pushed to main"

# --- an app change under a whole release needs a bump
echo '// change' >>src/main.rs
git commit -qam change
fails version release || bad "a changed app under a released version passed"
grep -q 'bump the version' "$tmp/out" || bad "the failure does not say to bump"
ok "an app change after a whole release says to bump the version"

# --- resume: the release is missing an archive
scratch
publish new >/dev/null 2>&1
grep -v 'x86_64' "$stub/assets" >"$stub/assets.new" && mv "$stub/assets.new" "$stub/assets"
version release | grep -q 'release=resume' || bad "a release missing an archive should resume"
: >"$stub/calls"
publish resume >/dev/null 2>&1 || bad "publish resume failed"
[ "$(grep -c 'release upload' "$stub/calls")" = 1 ] && grep -q 'release upload v0.1.0 .*x86_64' "$stub/calls" || bad "only the missing archive should be uploaded"
! grep -q 'release create' "$stub/calls" || bad "an existing release was created again"
ok "a release missing an archive is completed with just that archive"

# --- resume: the release commit never reached main
scratch
git config user.name test
base=$(git rev-parse HEAD)
.github/manifest.sh cornercase-plugin.toml 0.1.0 "$tmp/dist"
git add cornercase-plugin.toml && git -c user.name='github-actions[bot]' -c user.email='41898282+github-actions[bot]@users.noreply.github.com' commit -q -m "chore: release v0.1.0 [skip ci]"
git tag v0.1.0
git push -q origin v0.1.0
tagged=$(git rev-parse HEAD)
git reset -q --hard "$base"
version release | grep -q 'release=resume' || bad "a tag that never reached main should resume"
git checkout -q --detach v0.1.0
publish resume >/dev/null 2>&1 || bad "publish resume failed for a tag that never reached main"
[ "$(git -C "$tmp/origin.git" rev-parse main)" = "$tagged" ] || bad "the tagged commit did not reach main"
grep -q 'release create v0.1.0' "$stub/calls" || bad "the missing release was not created"
ok "a tag that never reached main is released and pushed to main"

# --- resume: the rebuilt archives are not the ones the manifest names
git checkout -q --detach v0.1.0
reset_stub
printf 'changed' >"$tmp/dist/cornercase-notch-aarch64-apple-darwin.tar.gz"
fails publish resume || bad "an archive that differs from the manifest's sha256 passed"
[ ! -f "$stub/release" ] || bad "a release was made from archives that differ from the manifest"
ok "resume refuses archives whose checksums differ from the manifest's"

# --- resume: a tag on a commit that is not a genuine release commit is never pushed to main
for kind in author source other-field; do
    scratch
    base=$(git rev-parse HEAD)
    .github/manifest.sh cornercase-plugin.toml 0.1.0 "$tmp/dist"
    bot="-c user.name=github-actions[bot] -c user.email=41898282+github-actions[bot]@users.noreply.github.com"
    case $kind in
        source) echo '// sneaky' >>src/main.rs; git add -A ;;
        other-field) sed -i.bak 's/^id = "x"/id = "y"/' cornercase-plugin.toml && rm cornercase-plugin.toml.bak ;;
    esac
    git add -A
    # shellcheck disable=SC2086
    if [ "$kind" = author ]; then git commit -q -m release; else git $bot commit -q -m release; fi
    git tag v0.1.0
    git push -q origin v0.1.0
    git reset -q --hard "$base"
    fails version release || bad "a tag on a $kind-tampered commit passed the decision"
    git checkout -q --detach v0.1.0
    fails publish resume || bad "a tag on a $kind-tampered commit was pushed to main"
    [ "$(git -C "$tmp/origin.git" rev-parse main)" = "$base" ] || bad "main moved for a $kind-tampered tag"
done
ok "a tag alone never drives a push to main"

# --- resume: main moved on
scratch
base=$(git rev-parse HEAD)
.github/manifest.sh cornercase-plugin.toml 0.1.0 "$tmp/dist"
git add cornercase-plugin.toml && git commit -q -m release && git tag v0.1.0
git reset -q --hard "$base"
echo '// more' >>src/main.rs
git commit -qam more
fails version release || bad "a tag off main with main moved on passed"
ok "a tag that main moved past fails loudly"

# --- version.sh check: only the release commit changes the manifest's version and downloads
scratch
publish new >/dev/null 2>&1
git switch -q -c bump
sed -i.bak 's/0.1.0/0.2.0/' Cargo.toml && rm Cargo.toml.bak
git commit -qam "bump Cargo.toml only"
fails version check main || true
grep -q "CHANGELOG" "$tmp/out" || bad "a bump without a changelog section passed"
printf '## 0.2.0\n\n- second\n' | cat - CHANGELOG.md >CHANGELOG.new && mv CHANGELOG.new CHANGELOG.md
git commit -qam changelog
version check main >/dev/null || bad "a pull request that bumps Cargo.toml and the changelog only failed"
sed -i.bak 's/^version = "0.1.0"/version = "0.2.0"/' cornercase-plugin.toml && rm cornercase-plugin.toml.bak
git commit -qam "manifest version"
fails version check main || bad "a pull request that changes the manifest's version passed"
grep -q "only the release commit" "$tmp/out" || bad "wrong failure: $(cat "$tmp/out")"
git reset -q --hard HEAD~1
sed -i.bak 's/^sha256 = "\(.\)/sha256 = "0\1/;s/^sha256 = "\(.\{64\}\).*"/sha256 = "\1"/' cornercase-plugin.toml && rm cornercase-plugin.toml.bak
git commit -qam "downloads"
fails version check main || bad "a pull request that changes the downloads passed"
grep -q "only the release commit" "$tmp/out" || bad "wrong failure: $(cat "$tmp/out")"
ok "only the release commit changes the manifest's version and downloads"

# --- version.sh check: the manifest's version equals its own download URLs
scratch
publish new >/dev/null 2>&1
sed -i.bak 's/^version = "0.1.0"/version = "0.3.0"/' cornercase-plugin.toml && rm cornercase-plugin.toml.bak
fails version check main || bad "a manifest version that differs from its downloads passed"
grep -q 'downloads' "$tmp/out" || bad "the failure does not mention the downloads"
ok "the manifest's version must equal its download URLs"
