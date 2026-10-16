#!/bin/sh
set -eu

APP="src Cargo.toml Cargo.lock rust-toolchain.toml cornercase-plugin.toml"

fail() {
    echo "::error::$1"
    exit 1
}

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
manifest=$(sed -n 's/^version = "\(.*\)"$/\1/p' cornercase-plugin.toml | head -n 1)
latest=$(git tag --list 'v*' --sort=-v:refname | head -n 1 | sed 's/^v//')

newer() {
    [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -n 1)" = "$1" ]
}

# $APP is left unquoted on purpose: it is a list of paths.
check() {
    [ "$manifest" = "$version" ] || fail "cornercase-plugin.toml says version $manifest but Cargo.toml says $version; keep them equal"
    # shellcheck disable=SC2086
    changed=$(git diff --name-only "$1"...HEAD -- $APP)
    if [ -z "$changed" ]; then
        echo "no app changes, no new version needed"
        return
    fi
    if [ -n "$latest" ] && ! newer "$version" "$latest"; then
        fail "this pull request changes the app, so it needs a new version: set version in Cargo.toml and cornercase-plugin.toml above $latest (the last release) and add its section to CHANGELOG.md"
    fi
    grep -q "^## $version\$" CHANGELOG.md || fail "add a '## $version' section to CHANGELOG.md with what changed for users"
    echo "this pull request releases $version"
}

# Prints release=yes or release=no (also to $GITHUB_OUTPUT when it is set).
release() {
    tag="v$version"
    answer=no
    if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
        # shellcheck disable=SC2086
        changed=$(git diff --name-only "$tag" HEAD -- $APP)
        [ -z "$changed" ] || fail "$tag is already released, but the app changed since; bump the version in a new pull request"
        echo "$tag is already released"
    else
        [ "$manifest" = "$version" ] || fail "cornercase-plugin.toml says version $manifest but Cargo.toml says $version"
        grep -q "^## $version\$" CHANGELOG.md || fail "CHANGELOG.md has no '## $version' section"
        echo "$tag is not released yet"
        answer=yes
    fi
    echo "release=$answer"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        echo "release=$answer" >>"$GITHUB_OUTPUT"
    fi
}

case "${1:-}" in
    check) check "${2:?usage: version.sh check <base>}" ;;
    release) release ;;
    *) fail "usage: version.sh check <base> | version.sh release" ;;
esac
