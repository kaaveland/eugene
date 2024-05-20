# `W12` Multiple `ALTER TABLE` statements where one will do

## Description

**Triggered when**: Multiple `ALTER TABLE` statements targets the same table.

**Effect**: If the statements require table scans, there will be more scans than necessary.

**Workaround**: Combine the statements into one, separating the action with commas.

**Detected by**: `eugene lint`

## Problematic migration

```sql
-- 1.sql

create table authors(id integer generated always as identity primary key, name text, email text);

-- 2.sql

set lock_timeout = '2s';
alter table authors alter column name set not null;
-- eugene: ignore E2, E4
alter table authors alter column email set not null;

```

## Safer migration

```sql
-- 1.sql

create table authors(id integer generated always as identity primary key, name text, email text);

-- 2.sql

set lock_timeout = '2s';
-- eugene: ignore E2
alter table authors
  alter column name set not null,
  alter column email set not null;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
