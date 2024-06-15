# `E11` Adding a `SERIAL` or `GENERATED ... STORED` column

## Description

**Triggered when**: A new column was added with a `SERIAL` or `GENERATED` type.

**Effect**: This blocks all table access until the table is rewritten.

**Workaround**: Can not be done without a table rewrite.

**Detected by**: `eugene lint`

## Problematic migration

```sql
-- 1.sql
create table prices (
    price int not null
);

-- 2.sql
set local lock_timeout = '2s';
alter table prices
    add column id serial;
```

## Safer migration

Currently, we don't know of a safe way to avoid this issue.

Report an issue at the [tracker](https://github.com/kaaveland/eugene) if
you know a way!

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
