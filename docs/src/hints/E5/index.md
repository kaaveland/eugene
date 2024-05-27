# `E5` Type change requiring table rewrite

## Description

**Triggered when**: A column was changed to a data type that isn't binary compatible.

**Effect**: This causes a full table rewrite while holding a lock that prevents all other use of the table.

**Workaround**: Add a new column, update it in batches, and drop the old column.

**Detected by**: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql

create table prices (
    id integer generated always as identity
        primary key,
    price int not null
);

-- 2.sql

set local lock_timeout = '2s';
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

-- 2.sql

set local lock_timeout = '2s';
alter table prices
    add column new_price bigint;

-- 3.sql

update prices set new_price = price :: bigint;
set local lock_timeout = '2s';
alter table prices
    add constraint check_new_price_not_null
        check (new_price is not null) not valid;

-- 4.sql

set local lock_timeout = '2s';
alter table prices
    validate constraint check_new_price_not_null,
    drop column price;
-- eugene: ignore E4
-- this has to run in the same transaction as dropping the old price column
alter table prices
    rename column new_price to price;

```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
