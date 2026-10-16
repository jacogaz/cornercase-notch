#!/bin/sh
# Publish a release from the archives in ./dist (or $DIST): publish.sh new | resume
#
#   new:    the archives were built from HEAD. Write the manifest, commit it as the release
#           commit, push the tag, create the GitHub release on it, and only then push the commit
#           to main (a fast-forward), so main never names an archive that does not exist.
#   resume: HEAD is the tagged release commit and the archives were built from it. Their
#           checksums must be the ones its manifest names. Create the release or the missing
#           archives, then push the tagged commit to main if it is not there (a fast-forward).
set -eu

. "$(dirname "$0")/lib.sh"

mode="${1:?usage: publish.sh new | resume}"
dist="${DIST:-dist}"
version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
tag="v$version"
targets="aarch64-apple-darwin x86_64-apple-darwin"

notes=$(mktemp "${TMPDIR:-/tmp}/notes.XXXXXX")
trap 'rm -f "$notes"' EXIT
awk -v heading="## $version" '$0 == heading { found = 1; next } /^## / { found = 0 } found' CHANGELOG.md >"$notes"

case "$mode" in
    new)
        .github/manifest.sh cornercase-plugin.toml "$version" "$dist"
        git config user.name 'github-actions[bot]'
        git config user.email '41898282+github-actions[bot]@users.noreply.github.com'
        git add cornercase-plugin.toml
        git commit -m "chore: release $tag [skip ci]"
        git tag "$tag"
        git push origin "$tag"
        gh release create "$tag" "$dist"/*.tar.gz --title "$tag" --notes-file "$notes" --verify-tag
        git push origin HEAD:refs/heads/main
        ;;
    resume)
        commit=$(git rev-parse HEAD)
        [ "$commit" = "$(git rev-parse "$tag^{commit}")" ] || fail "resume needs $tag checked out"
        for target in $targets; do
            archive="$dist/cornercase-notch-$target.tar.gz"
            expected=$(manifest_sha cornercase-plugin.toml "$target")
            [ -n "$expected" ] || fail "the manifest of $tag names no archive for $target"
            [ "$(sha256 "$archive")" = "$expected" ] || fail "the archive rebuilt from $tag for $target is not the one its manifest names ($expected); cannot resume safely"
        done
        if gh release view "$tag" >/dev/null 2>&1; then
            listed=$(gh release view "$tag" --json assets --jq '.assets[].name' 2>/dev/null || true)
            for target in $targets; do
                name="cornercase-notch-$target.tar.gz"
                printf '%s\n' "$listed" | grep -qx "$name" || gh release upload "$tag" "$dist/$name"
            done
        else
            gh release create "$tag" "$dist"/*.tar.gz --title "$tag" --notes-file "$notes" --verify-tag
        fi
        git fetch origin main
        if git merge-base --is-ancestor "$commit" origin/main; then
            echo "$tag is already on main"
        else
            genuine_release "$commit" origin/main || fail "$tag is not a release commit made by the release workflow on top of main; not pushing it to main"
            git push origin "$commit:refs/heads/main"
        fi
        ;;
    *) fail "usage: publish.sh new | resume" ;;
esac
