# `E8` Creating a new exclusion constraint

## Description

**Triggered when**: Found a new exclusion constraint.

**Effect**: This blocks all reads and writes to the table while the constraint index is being created.

**Workaround**: There is no safe way to add an exclusion constraint to an existing table.

**Detected by**: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table authors (id integer generated always as identity primary key, name text not null);

-- 2.sql

set local lock_timeout = '2s';
alter table authors add constraint authors_name_excl exclude (name with =);

```

## Safer migration

Currently, we don't know of a safe way to avoid this issue.

Report an issue at the [tracker](https://github.com/kaaveland/eugene) if
you know a way!

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
