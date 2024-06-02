# `E6` Creating a new index on an existing table

## Description

**Triggered when**: A new index was created on an existing table without the `CONCURRENTLY` keyword.

**Effect**: This blocks all writes to the table while the index is being created.

**Workaround**: Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`.

**Detected by**: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql

set local lock_timeout = '2s';
create index
    authors_name_idx on authors (name);

```

## Safer migration

```sql
-- 1.sql

create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql

create index concurrently
    authors_name_idx on authors (name);

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
