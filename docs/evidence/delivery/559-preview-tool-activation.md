# 559 — Preview cross-tool activation

Date: 2026-07-21

## Failure evidence

The first organic preview run `29779814693` reached all hosted build lanes.
Both Linux jobs installed current Zig 0.16.0 and cargo-zigbuild 0.23.0, then
failed before compilation because mise 2026.7.7 reported that no version was
activated for its `cargo-zigbuild` shim. `install_args` installs additional
tools but, without repository config, does not select them for subsequent
steps.

## Repair

The current mise-action 4.2.1 README documents its `mise_toml` input for an
ephemeral workflow configuration. The Linux lanes now declare and activate the
exact current versions there. This keeps the repository free of a general mise
toolchain decision while making the release inputs reproducible and executable.

## Verification

```text
actionlint 1.7.12 .github/workflows/*.yml
# exit 0
```

Dynamic Linux compilation, packaging, and attestation proof remains required
on the next eligible organic/manual run.

No external product influenced this CI-tooling repair.
