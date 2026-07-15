# Contributing

TableRock is currently in a research phase. Open an issue or proposal before
adding application code, dependencies, configuration schemas, or public APIs.

## Changes

- Use Conventional Commits and DCO sign-off (`git commit -s`).
- Keep one focused concern per pull request.
- Update research/roadmap/docs with decisions and behavior.
- Add tests proportional to safety and cross-module impact.
- Record dependency version, features, license, MSRV, and motivation.

## Reference provenance

TablePro, TablePlus, and Zedis are concepts-only references. Contributions must
not copy or adapt their source, tests, comments, identifiers, assets, text,
screenshots, geometry, colors, or key bindings.

When external product documentation informs a change, include:

```text
External concept: <broad behavior>
Public source: <documentation URL>
TableRock requirement: <research/issue link>
Implementation source: official protocol/library docs and TableRock tests
Copied code/assets/text: none
```

## Safety

- Do not include secrets, production endpoints, database contents, or captured
  credentials in fixtures, logs, screenshots, or issues.
- Enforce write policy and redaction below UI code.
- Treat unknown operations as writes and ambiguous write outcomes as unsafe to
  retry automatically.
