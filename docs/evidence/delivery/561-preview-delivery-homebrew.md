# Evidence 561: verified preview delivery and Homebrew tap

Date: 2026-07-21

## Decision

TableRock uses the `homebrew-parallax` pull-verification model. The public tap
owns a scheduled/manual workflow and updates itself with its repository-scoped
`github.token`; TableRock holds no cross-repository PAT. `homebrew-holla`
provided formula, alias, trust, and install conventions. `homebrew-tap`
provided supply-chain review rules. All three live repositories and their
upstream preview workflows were reread before seeding the tap.

Formula and cask tokens remain distinct:

- `tablerock@preview` aliases `Formula/tablerock-preview.rb`.
- `tablerock-app@preview` names the arm64 native cask.

## Product release proof

Source: `9e321389829e9b5f8dee52dac9e6d01bd8dba490`

Release: `Preview 0.1.0-preview.623+9e32138`

- Organic successful-Checks run: [29797453695](https://github.com/tailrocks/tablerock/actions/runs/29797453695)
- Manual-dispatch run: [29797918025](https://github.com/tailrocks/tablerock/actions/runs/29797918025)
- Both runs built four CLI archives plus the arm64 app archive.
- Rolling release exposes exactly five archives and five `.sha256` files.
- Every checksum matched after download.
- `gh attestation verify --repo tailrocks/tablerock` passed for all five
  archives.
- Linux targets use a glibc 2.17 baseline. Both Linux lanes, both Apple CLI
  lanes, and the Xcode 26 native-app lane passed.

The manual rebuild produced a different app ZIP digest from the organic build
for the same source. No reproducibility claim is made: the rolling release
replaces timestamped build output. The tap accepted only the current archive
after independently matching its checksum and GitHub provenance.

## Tap proof

Repository: [tailrocks/homebrew-tablerock](https://github.com/tailrocks/homebrew-tablerock)

- Public repository, default branch `main`.
- Actions enabled; default workflow permission is `write`.
- Seed commit `a378d83` carries DCO and Codex trailers.
- `Aliases/tablerock@preview` is Git mode `120000` and targets
  `../Formula/tablerock-preview.rb`.
- Formula and cask both carry the current 40-hex `source-sha` marker.
- Final verifier run: [29799133717](https://github.com/tailrocks/homebrew-tablerock/actions/runs/29799133717), green.
- Verifier checks exact ten-asset membership, five checksums, signer workflow,
  `refs/heads/main`, source digest, hosted-runner provenance, Linux executable
  identity, native executable presence, and plist version before rewriting.
- Checkout is pinned to current stable v7.0.1. Local actionlint v1.7.12 passed.

The workflow commits with the repository token through GitHub's contents API.
Generated commits use the GitHub Actions bot identity, matching DCO sign-off,
and carry the required Codex co-author trailer.

## Real arm64 macOS transcript

Host: arm64, macOS 26.5.2, Homebrew 6.0.12 development snapshot.

```text
brew tap tailrocks/tablerock
Tapped 1 cask and 1 formula

brew trust tailrocks/tablerock
Trusted tap: tailrocks/tablerock

brew install tablerock@preview
/opt/homebrew/Cellar/tablerock-preview/0.1.0-preview.623+9e32138

tablerock --version
tablerock 0.1.0-preview.623+9e32138

brew test tablerock@preview
PASS

brew install --cask tablerock-app@preview
tablerock-app@preview was successfully installed

plutil -extract TableRockPreviewVersion raw \
  /Applications/TableRock.app/Contents/Info.plist
0.1.0-preview.623+9e32138

codesign --verify --deep --strict /Applications/TableRock.app
PASS

xattr -dr com.apple.quarantine /Applications/TableRock.app
open -a /Applications/TableRock.app
PASS: process /Applications/TableRock.app/Contents/MacOS/TableRock
PASS: layer-0 TableRock window; LaunchServices name TableRock; bundle ID
      app.tablerock.TableRock

brew uninstall tablerock@preview
PASS

brew uninstall --cask tablerock-app@preview
PASS: /Applications/TableRock.app removed; cask purged

brew untap tailrocks/tablerock
PASS
```

`~/Library/Application Support/TableRock/profiles.db` already existed. The
test intentionally did not use `--zap`, because deleting an operator database
without explicit authority would be destructive. The cask declares that exact
directory in `zap trash:` for an operator-requested full removal.

## Audit notes and exclusions

Ruby syntax, YAML parsing, actionlint, formula audit, install, and formula test
passed. Strict cask audit reports two expected rolling-channel objections:
the stable download URL is unversioned and `preview` is a GitHub prerelease.
Changing to `sha256 :no_check` would remove the integrity anchor and is
rejected. Normal cask installation verifies the pinned SHA successfully.

Named exclusions remain unchanged: stable channel, cosign bundles, CycloneDX
SBOMs, x86_64 native app, Developer ID signing, notarization, and stapling.
The cask caveat truthfully states the ad-hoc/Gatekeeper state.
