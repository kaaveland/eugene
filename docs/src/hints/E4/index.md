# `E4` Running more statements after taking `AccessExclusiveLock`

## Description

Triggered when: A transaction that holds an `AccessExclusiveLock` started a new statement.

Effect: This blocks all access to the table for the duration of this statement.

A safer way is: Run this statement in a new transaction.

Detected by: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table authors (
    id integer generated always as identity primary key,
    name text not null
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors add column email text not null;
select count(*) from authors;

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
alter table authors add column email text not null;

-- 3.sql

select count(*) from authors;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
