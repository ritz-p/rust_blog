#!/usr/bin/env bash
set -euo pipefail

repo="repos/$GITHUB_REPOSITORY"
asset_name="$BUNDLE_NAME.tar.gz"
archive="artifact/$asset_name"
digest=$(sha256sum < "$archive")
digest="sha256:${digest%% *}"

is_current_release_head() {
    local head_sha
    head_sha=$(gh api "$repo/git/ref/heads/release" --jq '.object.sha') || exit 1
    [[ "$head_sha" == "$GITHUB_SHA" ]]
}

if ! is_current_release_head; then
    echo "Skipping superseded release commit $GITHUB_SHA"
    exit 0
fi

# GitHub ignores target_commitish when the tag already exists.
existing_ref=$(gh api "$repo/git/matching-refs/tags/$RELEASE_TAG" |
    jq -r --arg ref "refs/tags/$RELEASE_TAG" 'any(.[]; .ref == $ref)')
if [[ "$existing_ref" == true ]]; then
    tag_sha=$(gh api "$repo/commits/$RELEASE_TAG" --jq '.sha')
    if [[ "$tag_sha" != "$GITHUB_SHA" ]]; then
        echo "Tag $RELEASE_TAG points to $tag_sha instead of $GITHUB_SHA" >&2
        exit 1
    fi
fi

# The authenticated list includes unpublished drafts left by interrupted runs.
release=$(gh api --paginate "$repo/releases?per_page=100" |
    jq -cs --arg tag "$RELEASE_TAG" 'add | map(select(.tag_name == $tag)) | first // empty')

if [[ -z "$release" ]]; then
    release=$(gh api --method POST "$repo/releases" \
        -f tag_name="$RELEASE_TAG" -f target_commitish="$GITHUB_SHA" \
        -f name="$RELEASE_TAG" -f body="Export tools for commit $GITHUB_SHA." -F draft=true)
fi
release_id=$(jq -er '.id | numbers' <<< "$release")

if [[ $(jq -r '.draft' <<< "$release") == false ]]; then
    jq -e --arg name "$asset_name" \
        'any(.assets[]; .name == $name and .state == "uploaded" and (.digest | strings | test("^sha256:[a-f0-9]{64}$")))' \
        <<< "$release" > /dev/null
    echo "Release $RELEASE_TAG is already published."
    exit 0
fi

if [[ $(jq -r '.target_commitish' <<< "$release") != "$GITHUB_SHA" ]]; then
    echo "Draft $RELEASE_TAG does not target $GITHUB_SHA" >&2
    exit 1
fi
gh release upload "$RELEASE_TAG" "$archive" --clobber --repo "$GITHUB_REPOSITORY"
gh api "$repo/releases/$release_id/assets" |
    jq -e --arg name "$asset_name" --arg digest "$digest" \
        'any(.[]; .name == $name and .state == "uploaded" and .digest == $digest)' > /dev/null

if is_current_release_head; then
    gh api --method PATCH "$repo/releases/$release_id" \
        -F draft=false -F prerelease=false -f make_latest=true --silent
else
    echo "Keeping $RELEASE_TAG as a draft because release advanced during upload."
fi
