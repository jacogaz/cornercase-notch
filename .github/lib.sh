# Shared by the other scripts: source it.

fail() {
    echo "::error::$1"
    exit 1
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    else
        shasum -a 256 "$1" | cut -d ' ' -f 1
    fi
}

BOT_EMAIL="41898282+github-actions[bot]@users.noreply.github.com"

# The sha256 a manifest names for a target: manifest_sha <manifest> <target>
manifest_sha() {
    awk -v table="[download.$2]" '$0 == table { found = 1; next } /^\[/ { found = 0 } found && /^sha256 = / { gsub(/"/, "", $3); print $3 }' "$1"
}

REPO="jacogaz/cornercase-notch"
TARGETS="aarch64-apple-darwin x86_64-apple-darwin"

# Print a manifest as a release commit writes it: render_manifest <manifest> <version> <sha function>.
# The manifest's own text before its first [download.] table, its first `version = ` line set to the
# version, then one fixed table per target, each with the sha256 the function prints for the target.
render_manifest() {
    awk -v version="$2" '/^\[download\./ { exit } !done && /^version = / { print "version = \"" version "\""; done = 1; next } { print }' "$1" |
        sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'
    for target in $TARGETS; do
        sum=$("$3" "$target") || return 1
        case $sum in *[!0-9a-f]* | "") return 1 ;; esac
        [ "${#sum}" = 64 ] || return 1
        printf '\n[download.%s]\n' "$target"
        printf 'url = "https://github.com/%s/releases/download/v%s/cornercase-notch-%s.tar.gz"\n' "$REPO" "$2" "$target"
        printf 'sha256 = "%s"\n' "$sum"
    done
}

# Whether a commit is a genuine release commit that may be pushed to main: genuine_release <commit> <main>.
# Its parent is on main, it was made by the release bot, it changes nothing but the manifest, and the
# manifest is byte for byte what manifest.sh writes from the parent's manifest, the version in the
# commit's own downloads and the checksums it names. A tag alone never decides what reaches main.
genuine_release() {
    parent=$(git rev-parse "$1^") || return 1
    git merge-base --is-ancestor "$parent" "$2" || return 1
    [ "$(git log -1 --format=%ae "$1")" = "$BOT_EMAIL" ] || return 1
    [ "$(git diff --name-only "$parent" "$1")" = "cornercase-plugin.toml" ] || return 1
    work=$(mktemp -d "${TMPDIR:-/tmp}/genuine.XXXXXX") || return 1
    git show "$parent:cornercase-plugin.toml" >"$work/parent.toml" || return 1
    git show "$1:cornercase-plugin.toml" >"$work/commit.toml" || return 1
    version=$(sed -n 's#^url = ".*/releases/download/v\([^/]*\)/.*"$#\1#p' "$work/commit.toml" | sort -u)
    case $version in "" | *[!0-9a-zA-Z.+-]*) rm -rf "$work"; return 1 ;; esac
    candidate_sha() { manifest_sha "$work/commit.toml" "$1"; }
    render_manifest "$work/parent.toml" "$version" candidate_sha >"$work/expected.toml" || { rm -rf "$work"; return 1; }
    cmp -s "$work/expected.toml" "$work/commit.toml"
    status=$?
    rm -rf "$work"
    return "$status"
}
