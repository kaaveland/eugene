# `E7` Creating a new unique constraint

## Description

Triggered when: Found a new unique constraint and a new index.

Effect: This blocks all writes to the table while the index is being created and validated.

A safer way is: `CREATE UNIQUE INDEX CONCURRENTLY`, then add the constraint using the index.

Detected by: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table authors(
    id integer generated always as identity primary key,
    name text not null
);

-- 2.sql

set local lock_timeout = '2s';
alter table authors add constraint unique_name unique(name);

```

## Safer way

```sql
-- 1.sql

create table authors(
    id integer generated always as identity primary key,
    name text not null
);

-- 2.sql

create unique index concurrently authors_name_unique on authors(name);


-- 3.sql

set local lock_timeout = '2s';
alter table authors add constraint unique_name unique using index authors_name_unique;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
