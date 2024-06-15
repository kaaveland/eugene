# `E3` Add a new JSON column

## Description

**Triggered when**: A new column of type `json` was added to a table.

**Effect**: This breaks `SELECT DISTINCT` queries or other operations that need equality checks on the column.

**Workaround**: Use the `jsonb` type instead, it supports all use-cases of `json` and is more robust and compact.

**Detected by**: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null,
    meta json
);
```

## Safer migration

```sql
-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null,
    meta jsonb
);
```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
