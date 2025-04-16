# `E15` Missing index

## Description

**Triggered when**: A foreign key is missing a complete index on the referencing side.

**Effect**: Updates and deletes on the referenced table may cause table scan on referencing table.

**Workaround**: Create the missing index.

**Detected by**: `eugene trace`

## Problematic migration

```sql
-- 1.sql
create table items
(
    id bigint generated always as identity primary key
);

create table purchase
(
    id   bigint generated always as identity primary key,
    item bigint not null references items (id) -- no index
);

-- 2.sql
set local lock_timeout = '2s';
-- eugene: ignore E6
create index purchase_item_idx on purchase (item)
    -- this is a partial index, not good enough for enforcing referential integrity
    where item = 1;

```

## Safer migration

```sql
-- 1.sql
create table items
(
    id bigint generated always as identity primary key
);

create table purchase
(
    id   bigint generated always as identity primary key,
    item bigint not null references items (id)
);

-- 2.sql
set local lock_timeout = '2s';
-- eugene: ignore E6
create index purchase_item_idx on purchase(item);
```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
