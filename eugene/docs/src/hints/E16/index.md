# `E16` Running more statements after taking dangerous lock

## Description

**Triggered when**: A transaction that holds a dangerous lock started a new statement.

**Effect**: This blocks concurrent queries for the duration of this statement.

**Workaround**: Run this statement in a new transaction, or minimize the time spent holding dangerous locks.

**Detected by**: `eugene lint` and `eugene trace`

## Problematic migration

```sql
-- 1.sql
create table products (
    id integer generated always as identity primary key,
    name text not null,
    price decimal(10,2) not null
);

-- 2.sql
set local lock_timeout = '2s';

create or replace function products_trigger_fn()
    returns trigger as
$$
begin
    -- Simple trigger that logs changes
    raise notice 'Product % was modified', new.id;
    return new;
end;
$$ language plpgsql;

create trigger products_audit_trigger
    after insert or update
    on products
    for each row
execute function products_trigger_fn();

select count(*) from products where name = 'Widget';
```

## Safer migration

```sql
-- 1.sql
create table products
(
    id    integer generated always as identity primary key,
    name  text           not null,
    price decimal(10, 2) not null
);

-- 2.sql
set local lock_timeout = '2s';

create or replace function products_trigger_fn()
    returns trigger as
$$
begin
    raise notice 'Product % was modified', new.id;
    return new;
end;
$$ language plpgsql;

create trigger products_audit_trigger
    after insert or update
    on products
    for each row
execute function products_trigger_fn();

-- 3.sql
select count(*)
from products
where name = 'Widget';
```

## Eugene report examples

- [Problem linted by Eugene](unsafe_lint.md)
- [Problem traced by Eugene](unsafe_trace.md)
- [Fix linted by Eugene](safer_trace.md)
- [Fix traced by Eugene](safer_trace.md)
