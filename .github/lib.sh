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

# The sha256 a manifest names for a target: manifest_sha <manifest> <target>
manifest_sha() {
    awk -v table="[download.$2]" '$0 == table { found = 1; next } /^\[/ { found = 0 } found && /^sha256 = / { gsub(/"/, "", $3); print $3 }' "$1"
}

BOT_EMAIL="41898282+github-actions[bot]@users.noreply.github.com"

# The manifest without what only a release commit writes: its version and its download tables.
manifest_static() {
    git show "$1:cornercase-plugin.toml" | awk '/^\[download\./ { exit } !seen && /^version = / { seen = 1; next } { print }' | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'
}

# Whether a commit is a genuine release commit that may be pushed to main: genuine_release <commit> <main>.
# Its parent is on main, it was made by the release bot, and it changes nothing but the manifest's
# version and download tables. A tag alone never decides what reaches main.
genuine_release() {
    parent=$(git rev-parse "$1^") || return 1
    git merge-base --is-ancestor "$parent" "$2" || return 1
    [ "$(git log -1 --format=%ae "$1")" = "$BOT_EMAIL" ] || return 1
    [ "$(git diff --name-only "$parent" "$1")" = "cornercase-plugin.toml" ] || return 1
    [ "$(manifest_static "$parent")" = "$(manifest_static "$1")" ] || return 1
}
