## ✅ Eugene trace report

Script name: `examples/E16/good/1.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 1.sql
create table products
(
    id    integer generated always as identity primary key,
    name  text           not null,
    price decimal(10, 2) not null
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


## ✅ Eugene trace report

Script name: `examples/E16/good/2.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 2.sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ✅ Statement number 2 for 10ms

```sql
create or replace function products_trigger_fn()
    returns trigger as
$$
begin
    raise notice 'Product % was modified', new.id;
    return new;
end;
$$ language plpgsql
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ✅ Statement number 3 for 10ms

```sql
create trigger products_audit_trigger
    after insert or update
    on products
    for each row
execute function products_trigger_fn()
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `products` | `ShareRowExclusiveLock` | Table | 1 | ❌ | 10 |


## ✅ Eugene trace report

Script name: `examples/E16/good/3.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 3.sql
select count(*)
from products
where name = 'Widget'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.

