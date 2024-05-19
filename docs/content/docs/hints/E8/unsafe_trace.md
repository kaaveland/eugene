---

title:  Traced matching transaction
weight: 50
---


## Eugene 🔒 trace report of `examples/E8/bad/1.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create table authors (id integer generated always as identity primary key, name text not null)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E8/bad/2.sql`

### Statement number 1 for 10 ms

### SQL

```sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


### Statement number 2 for 10 ms

### SQL

```sql
alter table authors add constraint authors_name_excl exclude (name with =)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |
| `public` | `authors` | `ShareLock` | Table | 1 | ❌ |

### Hints

##### Creating a new index on an existing table

ID: `E6`

A new index was created on an existing table without the `CONCURRENTLY` keyword. This blocks all writes to the table while the index is being created. A safer way is: Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`.

A new index was created on the table `public.authors`. The index `public.authors_name_excl` was created non-concurrently, which blocks all writes to the table. Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.

##### Creating a new exclusion constraint

ID: `E8`

Found a new exclusion constraint. This blocks all reads and writes to the table while the constraint index is being created. A safer way is: There is no safe way to add an exclusion constraint to an existing table.

A new exclusion constraint `authors_name_excl` was added to the table `public.authors`. There is no safe way to add an exclusion constraint to an existing table. This constraint creates an index on the table, and blocks all reads and writes.

