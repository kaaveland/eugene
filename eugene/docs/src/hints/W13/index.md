# `W13` Creating an enum

## Description

**Triggered when**: A new enum was created.

**Effect**: Removing values from an enum requires difficult migrations, and associating more data with an enum value is difficult.

**Workaround**: Use a foreign key to a lookup table instead.

**Detected by**: `eugene lint`

## Problematic migration

```sql
-- 1.sql

create type document_type
    as enum ('invoice', 'receipt', 'other');
create table document (
    id int generated always as identity
        primary key,
    type document_type
);

```

## Safer migration

```sql
-- 1.sql

create table document_type(
    type_name text primary key
);
insert into document_type
  values('invoice'), ('receipt'), ('other');
create table document (
    id int generated always as identity
        primary key,
    type text
        references document_type(type_name)
);

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
