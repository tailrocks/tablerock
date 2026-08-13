# Contributing

[`AGENTS.md`](AGENTS.md) is the authoritative rule set for this repository;
this file is the contributor-facing summary. Before adding application code,
dependencies, configuration schemas, or public APIs, confirm the relevant
[roadmap](ROADMAP.md) checkpoint and its adoption requirements are approved.

## Pull-request delivery

- Never push directly to `main`. Use the one branch and pull request authorized
  by [`AGENTS.md`](AGENTS.md); do not create a duplicate or concurrent branch or
  pull request for the same goal.
- Keep published branch history forward-only. Merge current `main` into the
  branch when it falls behind; do not force-push or rewrite published history.
- Keep each checkpoint commit focused, buildable, and safe to publish.
- Use a Conventional Commit subject, DCO sign-off (`git commit -s`), and the
  `Co-authored-by: Codex <codex@openai.com>` trailer for Codex-authored work.
- Run the checks required by the changed surface before committing, then push
  the commit immediately unless the operator explicitly says to hold it.
- Required TermRock changes follow the same reviewed delivery rule, land before
  TableRock updates its exact revision pin, and remain product-neutral. Jackin
  is never modified as part of TableRock delivery.

## Changes

- Keep one focused concern per checkpoint commit.
- Update architecture docs, evidence, and the roadmap with decisions and
  behavior; new completed checkpoints add one numbered document under
  `docs/evidence/` plus one line in the [evidence
  index](docs/evidence/README.md).
- Add tests proportional to safety and cross-module impact.
- Record dependency version, features, license, MSRV, and motivation.

## Reference provenance

TablePro, TablePlus, and Zedis may inform product ideas, workflows, interaction
design, screen composition, and visual direction. Contributions must not copy,
translate, or derive implementation from their source code, tests, or source
comments. Third-party branding and proprietary assets require clear permission.

When external product documentation informs a change, include:

```text
External concept: <broad behavior>
Public source: <documentation URL>
TableRock requirement: <research/issue link>
Implementation source: official protocol/library docs and TableRock tests
Copied source code: none
```

## Safety

- Do not include secrets, production endpoints, database contents, or captured
  credentials in fixtures, logs, screenshots, or issues.
- Enforce write policy and redaction below UI code.
- Treat unknown operations as writes and ambiguous write outcomes as unsafe to
  retry automatically.
