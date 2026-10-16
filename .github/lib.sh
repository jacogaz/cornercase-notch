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
