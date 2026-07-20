# 542 — actions/checkout 7.0.1 maintenance

Date: 2026-07-21

## Trigger

The current `main` Dependencies run 29772815389 failed its intentional
latest-stable gate because `actions/checkout` 7.0.1 superseded 7.0.0.

## Change

All checkout uses in `checks.yml` and `dependencies.yml` now pin the full
v7.0.1 commit SHA `3d3c42e5aac5ba805825da76410c181273ba90b1` and retain the version comment.
The freshness assertion checks both the latest release tag and that tag's
resolved commit.

Primary source: GitHub API release and commit endpoints for
`actions/checkout`. Copied code/assets/text: none.

## Verification

```text
gh api repos/actions/checkout/releases/latest --jq .tag_name
# v7.0.1
gh api repos/actions/checkout/commits/v7.0.1 --jq .sha
# 3d3c42e5aac5ba805825da76410c181273ba90b1
```

The push run is the authoritative workflow verification.
