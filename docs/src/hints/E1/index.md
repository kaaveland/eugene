# `E1` Validating table with a new constraint

## Description

**Triggered when**: A new constraint was added and it is already `VALID`.

**Effect**: This blocks all table access until all rows are validated.

**Workaround**: Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later.

**Detected by**: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table authors(
    id integer generated always as identity
        primary key,
    name text
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors
    add constraint name_not_null
        check (name is not null);

```

## Safer migration

```sql
-- 1.sql

create table authors(
    id integer generated always as identity
        primary key,
    name text
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors
    add constraint name_not_null
        check (name is not null) not valid;

-- 3.sql

set local lock_timeout = '2s';
alter table authors
    validate constraint name_not_null;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
