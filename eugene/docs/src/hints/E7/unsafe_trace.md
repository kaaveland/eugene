## ✅ Eugene trace report

Script name: `examples/E7/bad/1.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 1.sql
create table authors(
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

Script name: `examples/E7/bad/2.sql`


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
alter table authors
    add constraint unique_name unique(name)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |
| `public` | `authors` | `ShareLock` | Table | 1 | ❌ | 10 |

#### Triggered rules

##### `E6`: [Creating a new index on an existing table](https://kaveland.no/eugene/hints/E6/)

A new index was created on the table `public.authors`. The index `public.unique_name` was created non-concurrently, which blocks all writes to the table. Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.

##### `E7`: [Creating a new unique constraint](https://kaveland.no/eugene/hints/E7/)

A new unique constraint `unique_name` was added to the table `public.authors`. This constraint creates a unique index on the table, and blocks all writes. Consider creating the index concurrently in a separate transaction, then adding the unique constraint by using the index: `ALTER TABLE public.authors ADD CONSTRAINT unique_name UNIQUE USING INDEX public.unique_name;`
