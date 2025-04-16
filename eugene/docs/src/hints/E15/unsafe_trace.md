## ✅ Eugene trace report

Script name: `examples/E15/bad/1.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 1.sql
create table items
(
    id bigint generated always as identity primary key
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ✅ Statement number 2 for 10ms

```sql
create table purchase
(
    id   bigint generated always as identity primary key,
    item bigint not null references items (id) -- no index
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


## ❌ Eugene trace report

Script name: `examples/E15/bad/2.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 2.sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ❌ Statement number 2 for 10ms

```sql
-- eugene: ignore E6
create index purchase_item_idx on purchase (item)
    -- this is a partial index, not good enough for enforcing referential integrity
    where item = 1
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `purchase` | `ShareLock` | Table | 1 | ❌ | 10 |

#### Triggered rules

##### `E15`: [Missing index](https://kaveland.no/eugene/hints/E15/)

Missing index for `purchase_item_fkey` detected, create it with:
```sql
create index concurrently "purchase_item_fkey_idx" on "public"."purchase"(item);
```

