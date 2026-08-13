# External Product Reference Policy

TableRock may study external database clients to improve product direction and
interface quality. Public documentation, screenshots, product behavior,
workflows, interaction patterns, screen composition, and visual concepts may
inform TableRock design. Common native macOS patterns may be adapted directly
through Apple's platform components and guidance.

## Source-code boundary

Never read external application source code for implementation guidance. Never
copy, translate, structurally port, or derive TableRock implementation from
external source code, tests, or source comments. Implement behavior from
TableRock requirements, official database and library documentation, Apple
platform documentation, and TableRock-owned tests.

Publicly observable product ideas and design concepts may be adapted when they
serve TableRock. External branding and proprietary assets remain excluded
unless their license or owner clearly permits reuse.

## References

### TablePro

TablePro is the primary external reference for database-workbench workflows and
native macOS interface direction. Its public documentation, product behavior,
and screenshots may inform requirements and design. Its AGPL source code,
tests, and source comments must not inform or enter TableRock implementation.

### TablePlus

TablePlus may inform simplicity, density, navigation, and native interaction
ideas through publicly observable behavior and documentation. Its proprietary
implementation must not be copied or reverse engineered.

### Zedis

Zedis may inform Redis workflows and typed-keyspace interaction concepts. Its
source code and tests must not be used as implementation material.

## Provenance

When an external product materially influences a change, record:

```text
External reference: <product and public URL>
Observed idea: <workflow, interaction, or design concept>
TableRock adaptation: <requirement or design decision>
Implementation sources: <official platform/database/library docs and TableRock tests>
Copied source code: none
```

Historical evidence written under the former stricter policy remains valid as
a record of how those checkpoints were produced.
