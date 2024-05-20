# `E9` Taking dangerous lock without timeout

## Description

Triggered when: A lock that would block many common operations was taken without a timeout.

Effect: This can block all other operations on the table indefinitely if any other transaction holds a conflicting lock while `idle in transaction` or `active`.

A safer way is: Run `SET LOCAL lock_timeout = '2s';` before the statement and retry the migration if necessary.

Detected by: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table authors (
    id integer generated always as identity primary key,
    name text not null
);

-- 2.sql

alter table authors add column email text;

```

## Safer way

```sql
-- 1.sql

create table authors (
    id integer generated always as identity primary key,
    name text not null
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors add column email text;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
