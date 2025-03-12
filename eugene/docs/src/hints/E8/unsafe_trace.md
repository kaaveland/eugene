## ✅ Eugene trace report

Script name: `examples/E8/bad/1.sql`


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

Script name: `examples/E8/bad/2.sql`


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
    add constraint authors_name_excl
        exclude (name with =)
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

A new index was created on the table `public.authors`. The index `public.authors_name_excl` was created non-concurrently, which blocks all writes to the table. Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.

##### `E8`: [Creating a new exclusion constraint](https://kaveland.no/eugene/hints/E8/)

A new exclusion constraint `authors_name_excl` was added to the table `public.authors`. There is no safe way to add an exclusion constraint to an existing table. This constraint creates an index on the table, and blocks all reads and writes.
