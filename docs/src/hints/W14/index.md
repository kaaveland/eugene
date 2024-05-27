# `W14` Adding a primary key using an index

## Description

**Triggered when**: A primary key was added using an index on the table.

**Effect**: This can cause postgres to alter the index columns to be `NOT NULL`.

**Workaround**: Make sure that all the columns in the index are already `NOT NULL`.

**Detected by**: `eugene lint`

## Problematic migration

```sql
-- 1.sql

create table authors(
    name text
);

-- 2.sql

create unique index concurrently
    authors_name_key on authors(name);

-- 3.sql

set local lock_timeout = '2s';
alter table authors
    add constraint authors_name_pkey
        primary key using index authors_name_key;

```

## Safer migration

```sql
-- 1.sql

create table authors(
    name text
);

-- 2.sql

create unique index concurrently
    authors_name_key on authors(name);

-- 3.sql

set local lock_timeout = '2s';
-- eugene: ignore E2
-- This is a demo of W14, so we can ignore E2 instead of the
-- multi-step migration to make the column NOT NULL safely
alter table authors
    alter column name set not null;

-- 4.sql

alter table authors
    add constraint authors_name_pkey
        primary key using index authors_name_key;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
