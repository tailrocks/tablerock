# Connections

The connection experience is the entry point of both clients. Running
`tablerock` or launching the macOS app opens the connection list.

## Connection list

The list shows every saved profile with:

- name and engine badge (PostgreSQL, ClickHouse, Redis);
- target summary: `host:port/database` (Redis: logical database index);
- environment tag (see below), shown as label plus color, never color alone;
- safety mode: Read only or Confirm writes;
- secret-source warning where the password is stored as acknowledged
  plaintext;
- live state: disconnected, connecting, connected, reconnecting, failed.

Visible actions: **Connect**, **New connection**, **Edit**, **Duplicate**,
**Test**, **Remove**. Empty, loading, and failed states are explicit screens,
not blank areas.

Search filters by name, host, database, and group. Filtering preserves group
structure.

### Groups

Connections organize into named groups (for example `production`,
`staging`, `local`). Groups are collapsible sections in the list; a profile
belongs to at most one group. Group operations: create, rename, delete
(connections move to ungrouped, never cascade-delete), assign on create/edit.
Ordering inside a group is manual or alphabetical.

### Environment tags

Every profile carries an optional environment tag: `production`, `staging`,
`development`, `testing`, or a custom label. The tag follows the connection
everywhere: list row, editor, workbench context bar, and every tab of that
session. `production` renders as a persistent warning treatment in addition to
its label.

## Connection editor

The first editor version has exactly these fields:

| Section | Fields |
|---|---|
| General | engine, name, group, environment tag |
| Connection | host, port, default database (Redis: logical DB index) |
| Credentials | username, password |
| TLS | mode: off, verify CA, verify full; custom CA later |

Nothing else is present in the first version: no SSH, no per-field secret
mapping, no advanced engine options. Those land in later phases and extend
this form, they do not redesign it.

### Password

There is exactly **one password configuration** per profile: a single
password field with one selected storage source. Initial sources, staged:

1. **Prompt on connect** — never stored; asked on every Test/Connect.
2. **Save locally (dangerous)** — stored plaintext on disk; requires an
   explicit acknowledgement and renders a persistent warning everywhere the
   profile appears.
3. Later: **1Password reference**, **macOS Keychain** (native client),
   **environment variable**.

A resolved password exists only during Test/Connect. It never enters
snapshots, logs, history, telemetry, or FFI events.

### Test and Connect

**Test** runs without saving and reports: server identity/version, TLS
outcome, elapsed time, and a redacted failure reason on error. **Connect**
saves (or uses a temporary session) and opens the workbench.

**Temporary connection**: connect without persisting profile or secret;
nothing durable remains after quit.

## Both clients

| | TUI | Native macOS |
|---|---|---|
| List | TermRock `Tree` (groups) + `List`, search input | SwiftUI `List` with sections, search field |
| Editor | TermRock `Form`, sections, focus traversal | SwiftUI `Form` scene, native controls |
| Password prompt | modal dialog | native secure field sheet |
| Warnings | text + glyph, never color alone | label + symbol, never color alone |

## States and failure truth

- Connection failure shows the redacted reason; retry is explicit.
- A profile with an unresolved or failing secret source fails before any
  network I/O, with the source named, never its value.
- Reconnect uses bounded backoff and stops on authentication failure.
- Removing a profile never silently removes unrelated history or active
  sessions; it asks when either exists.

## Deferred to later phases

1Password field mapping, custom CA/mTLS editing UI. URL import, external URL
open, SSH tunnel, and startup actions landed (see delivery evidence 260–274,
289, 296). Each extension is tracked in the parity ledger.
