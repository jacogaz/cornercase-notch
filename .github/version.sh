#!/bin/sh
set -eu

. "$(dirname "$0")/lib.sh"

APP="src Cargo.toml Cargo.lock rust-toolchain.toml cornercase-plugin.toml"
ARCHIVES="cornercase-notch-aarch64-apple-darwin.tar.gz cornercase-notch-x86_64-apple-darwin.tar.gz"

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
manifest=$(sed -n 's/^version = "\(.*\)"$/\1/p' cornercase-plugin.toml | head -n 1)
latest=$(git tag --list 'v*' --sort=-v:refname | head -n 1 | sed 's/^v//')

newer() {
    [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -n 1)" = "$1" ]
}

# What only the release commit may change in the manifest: its version and its download tables.
release_part() {
    git show "$1:cornercase-plugin.toml" 2>/dev/null | awk '/^version = / && !seen { print; seen = 1 } /^\[download\./ { tables = 1 } tables'
}

# $APP is left unquoted on purpose: it is a list of paths.
check() {
    urls=$(sed -n 's#^url = ".*/releases/download/v\([^/]*\)/.*"$#\1#p' cornercase-plugin.toml | sort -u)
    if [ -n "$urls" ] && [ "$urls" != "$manifest" ]; then
        fail "the manifest says version $manifest but its downloads are for $urls; only the release commit changes them"
    fi
    if git cat-file -e "$1:cornercase-plugin.toml" 2>/dev/null; then
        [ "$(release_part "$1")" = "$(release_part HEAD)" ] || fail "this pull request changes the manifest's version or downloads; only the release commit does, so bump Cargo.toml and CHANGELOG.md only"
    fi
    # shellcheck disable=SC2086
    changed=$(git diff --name-only "$1"...HEAD -- $APP)
    if [ -z "$changed" ]; then
        echo "no app changes, no new version needed"
        return
    fi
    if [ -n "$latest" ] && ! newer "$version" "$latest"; then
        fail "this pull request changes the app, so it needs a new version: set version in Cargo.toml above $latest (the last release) and add its section to CHANGELOG.md"
    fi
    grep -q "^## $version\$" CHANGELOG.md || fail "add a '## $version' section to CHANGELOG.md with what changed for users"
    echo "this pull request releases $version"
}

# Prints release=new, resume or no (also to $GITHUB_OUTPUT when it is set), and the version.
#   new:    there is no tag yet.
#   resume: the tag exists but the release is not whole (the release commit never reached main,
#           or the GitHub release or one of its archives is missing); finish it from the tag.
#   no:     the release is whole; fails if the app changed since it, which needs a new version.
release() {
    tag="v$version"
    answer=new
    if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
        commit=$(git rev-parse "$tag^{commit}")
        listed=$(gh release view "$tag" --json assets --jq '.assets[].name' 2>/dev/null || true)
        whole=yes
        for archive in $ARCHIVES; do
            printf '%s\n' "$listed" | grep -qx "$archive" || whole=no
        done
        if git merge-base --is-ancestor "$commit" HEAD; then
            if [ "$whole" = yes ]; then
                # shellcheck disable=SC2086
                changed=$(git diff --name-only "$tag" HEAD -- $APP)
                [ -z "$changed" ] || fail "$tag is already released, but the app changed since; bump the version in a new pull request"
                echo "$tag is already released"
                answer=no
            else
                echo "$tag is on main but its GitHub release is not whole; resuming"
                answer=resume
            fi
        else
            git merge-base --is-ancestor HEAD "$commit" || fail "$tag was made on a commit that never reached main and main has moved on; delete the tag and its release, or bump the version"
            echo "$tag's release commit never reached main; resuming"
            answer=resume
        fi
    else
        grep -q "^## $version\$" CHANGELOG.md || fail "CHANGELOG.md has no '## $version' section"
        echo "$tag is not released yet"
    fi
    echo "release=$answer"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        echo "release=$answer" >>"$GITHUB_OUTPUT"
        echo "version=$version" >>"$GITHUB_OUTPUT"
    fi
}

case "${1:-}" in
    check) check "${2:?usage: version.sh check <base>}" ;;
    release) release ;;
    *) fail "usage: version.sh check <base> | version.sh release" ;;
esac
