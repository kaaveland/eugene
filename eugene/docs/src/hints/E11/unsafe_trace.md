## ✅ Eugene trace report

Script name: `examples/E11/bad/1.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 1.sql
create table prices (
    price int not null
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


## ❌ Eugene trace report

Script name: `examples/E11/bad/2.sql`


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
alter table prices
    add column id serial
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `prices` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |
| `public` | `prices` | `ShareLock` | Table | 1 | ❌ | 10 |

#### Triggered rules

##### `E6`: [Creating a new index on an existing table](https://kaveland.no/eugene/hints/E6/)

A new index was created on the table `public.prices`. The index was created non-concurrently, which blocks all writes to the table. Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.

##### `E10`: [Rewrote table or index while holding dangerous lock](https://kaveland.no/eugene/hints/E10/)

The Table `public.prices` was rewritten while holding `AccessExclusiveLock` on the Table `public.prices`. This blocks `SELECT`, `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE`, `UPDATE`, `DELETE`, `INSERT`, `MERGE` while the rewrite is in progress.
