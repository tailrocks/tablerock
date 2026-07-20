# 557 — Rolling preview workflow foundation

Date: 2026-07-21

## Drift and platform gates

- Current public `tailrocks/holla` and `tailrocks/parallax` preview workflows
  were re-read from `main`; TableRock uses parallax's pull-verified tap model
  and holla's compact rolling-release update shape.
- GitHub's current hosted-runner reference lists standard ARM64 `macos-26`.
  Runner image inventory `20260630.0313.1` contains Xcode 26.6 at
  `/Applications/Xcode_26.6.app` plus every stable Xcode 26 line back to 26.0.1.
- Plan math was corrected before implementation: four CLI archives plus one app
  archive and their five checksum files means 10 release assets; five archives
  receive GitHub build-provenance attestations.
- The preceding red Linux baseline was structurally repaired and its Ubuntu
  container-free job passed before this workflow checkpoint.

## Workflow contract

`Publish Homebrew Preview` accepts only a manual dispatch from `main` or a
successful same-repository push-triggered `Checks` completion on `main`.
Source classification falls open when the future tap is absent/unreadable and
tracks every release-input path. GitHub server-side commit count plus short SHA
forms `0.1.0-preview.N+abcdef0`.

The build matrix produces:

- arm64/x86_64 Apple CLI archives on GitHub macOS;
- arm64/x86_64 glibc-2.17 Linux CLI archives through zigbuild;
- an arm64 macOS 26 application zip through the shipped native builder and
  Xcode 26.6;
- one SHA-256 sidecar and hosted-runner provenance attestation per archive.

Publishing verifies the exact set and checksums, proves the source commit is on
`main`, then creates or atomically updates rolling prerelease `preview`.
Permissions are read-only globally and widen only to attestation identity or
release contents on the owning jobs.

## Static verification

```text
ruby -e 'require "yaml"; Dir[".github/workflows/*.yml"].each { |p| YAML.parse_file(p) }'
# workflow YAML parsed

actionlint 1.7.12 .github/workflows/*.yml
# exit 0 (official release archive checksum verified before execution)

# every distinct action repository referenced by workflows has a freshness
# assertion in dependencies.yml
# all action repos covered
```

Pins were resolved against current upstream releases on 2026-07-21:
checkout 7.0.1, upload-artifact 7.0.1, download-artifact 8.0.1,
attest-build-provenance 4.1.1, mise-action 4.2.1, and stable rust-toolchain.

Dynamic dispatch/release evidence remains required after push. Tap-side pull
verification remains Step 5 and does not yet exist.

No TablePro workflow or visual expression influenced this CI-only checkpoint.
