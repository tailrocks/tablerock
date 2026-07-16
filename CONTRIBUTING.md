# Contributing

TableRock is currently in a research phase. Record and approve the relevant
research decision before adding application code, dependencies, configuration
schemas, or public APIs.

## Trunk-only delivery

- Work directly on `main`; do not create or switch to another branch.
- Do not open pull requests.
- Keep each checkpoint commit focused, buildable, and safe to publish.
- Use a Conventional Commit subject, DCO sign-off (`git commit -s`), and the
  `Co-authored-by: Codex <codex@openai.com>` trailer for Codex-authored work.
- Run the checks required by the changed surface before committing, then push
  the commit immediately unless the operator explicitly says to hold it.
- Never force-push or rewrite published `main`; fix mistakes forward.
- When a reusable component/API is missing, implement, test, document, commit,
  and push it directly to TermRock `main` with no branch or pull request; then
  pin that exact revision from TableRock `main`. Jackin is never modified as
  part of TableRock delivery.

## Changes

- Keep one focused concern per checkpoint commit.
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
