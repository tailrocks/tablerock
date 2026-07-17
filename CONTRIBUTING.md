# Contributing

[`AGENTS.md`](AGENTS.md) is the authoritative rule set for this repository;
this file is the contributor-facing summary. Before adding application code,
dependencies, configuration schemas, or public APIs, confirm the relevant
[roadmap](ROADMAP.md) checkpoint and its adoption requirements are approved.

## Trunk-only delivery

- Work directly on `main`; do not create or switch to another branch.
- Do not open pull requests. Never force-push or rewrite published `main`; fix
  mistakes forward.
- Keep each checkpoint commit focused, buildable, and safe to publish.
- Use a Conventional Commit subject, DCO sign-off (`git commit -s`), and the
  `Co-authored-by: Codex <codex@openai.com>` trailer for Codex-authored work.
- Run the checks required by the changed surface before committing, then push
  the commit immediately unless the operator explicitly says to hold it.
- When a reusable component/API is missing, implement, test, document, commit,
  and push it directly to TermRock `main` with no branch or pull request; then
  pin that exact revision from TableRock `main`. Jackin is never modified as
  part of TableRock delivery.

## Changes

- Keep one focused concern per checkpoint commit.
- Update architecture docs, evidence, and the roadmap with decisions and
  behavior; new completed checkpoints add one numbered document under
  `docs/evidence/` plus one line in the [evidence
  index](docs/evidence/README.md).
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
