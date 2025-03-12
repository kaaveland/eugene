## ✅ Eugene trace report

Script name: `examples/E9/bad/1.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


## ❌ Eugene trace report

Script name: `examples/E9/bad/2.sql`


### ❌ Statement number 1 for 10ms

```sql
-- 2.sql
alter table authors add column email text
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |

#### Triggered rules

##### `E9`: [Taking dangerous lock without timeout](https://kaveland.no/eugene/hints/E9/)

The statement took `AccessExclusiveLock` on the Table `public.authors` without a timeout. It blocks `SELECT`, `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE`, `UPDATE`, `DELETE`, `INSERT`, `MERGE` while waiting to acquire the lock.
