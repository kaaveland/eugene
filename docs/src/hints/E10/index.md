# `E10` Rewrote table or index while holding dangerous lock

## Description

**Triggered when**: A table or index was rewritten while holding a lock that blocks many operations.

**Effect**: This blocks many operations on the table or index while the rewrite is in progress.

**Workaround**: Build a new table or index, write to both, then swap them.

**Detected by**: `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table prices (
    id integer generated always as identity
        primary key,
    price int not null
);

create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors add column meta jsonb;

-- eugene: ignore E5, E4
-- causes table rewrite, but this example isnt't about that
alter table prices
    alter price set data type bigint;

```

## Safer migration

```sql
-- 1.sql

create table prices (
    id integer generated always as identity
        primary key,
    price int not null
);

create table authors (
    id integer generated always as identity
        primary key,
    name text not null
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors
    add column meta jsonb;

-- 3.sql

set local lock_timeout = '2s';
-- eugene: ignore E5, E4
-- causes table rewrite, but this example isnt't about that
alter table prices
    alter price set data type bigint;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
