# `E2` Validating table with a new `NOT NULL` column

## Description

**Triggered when**: A column was changed from `NULL` to `NOT NULL`.

**Effect**: This blocks all table access until all rows are validated.

**Workaround**: Add a `CHECK` constraint as `NOT VALID`, validate it later, then make the column `NOT NULL`.

**Detected by**: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table authors(
    id integer generated always as identity primary key,
    name text
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors alter column name set not null;

```

## Safer migration

```sql
-- 1.sql

create table authors(
    id integer generated always as identity primary key,
    name text
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors add constraint check_name_not_null check (name is not null) not valid;

-- 3.sql

set local lock_timeout = '2s';
alter table authors validate constraint check_name_not_null;

-- 4.sql

set local lock_timeout = '2s';
-- eugene trace knows name has a valid not null check, but eugene lint doesn't
-- eugene: ignore E2
alter table authors alter name set not null;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
