# 550 — Persistence startup lease release

Date: 2026-07-21

## Failure and repair

Ubuntu run `29775391243` exposed a startup race. A corrupt database correctly
failed closed, but `PersistenceActor::open` detached the failed worker before
its `PathLease` destructor ran. An immediate operator retry after removing the
bad file could therefore receive false `DatabaseBusy`.

Failed/disconnected startup now joins the worker before returning. Thus every
startup error establishes that database objects and the process-local path
lease are already released. The actor still never rewrites a corrupt file.

## Verification

`corrupt_files_fail_closed_without_becoming_new_databases` passed ten
consecutive open-fail-remove-recreate trials. The pushed Ubuntu matrix is the
cross-platform authority.

No external product influenced this ownership repair.
