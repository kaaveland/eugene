## Eugene 🔒 trace report of `examples/E10/bad/1.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create table prices (id integer generated always as identity primary key, price int not null)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


### Statement number 2 for 10 ms

### SQL

```sql
create table authors (id integer generated always as identity primary key, name text not null)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E10/bad/2.sql`

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
alter table authors add column meta jsonb
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |

### Statement number 3 for 10 ms

### SQL

```sql
-- eugene: ignore E5, E4
-- causes table rewrite, but this example isnt't about that
alter table prices alter price set data type bigint
```

#### Locks at start

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `prices` | `AccessExclusiveLock` | Table | 1 | ❌ |
| `public` | `prices` | `ShareLock` | Table | 1 | ❌ |
| `public` | `prices_pkey` | `AccessExclusiveLock` | Index | 1 | ❌ |

### Hints

##### Creating a new index on an existing table

ID: `E6`

A new index was created on an existing table without the `CONCURRENTLY` keyword. This blocks all writes to the table while the index is being created. A safer way is: Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`.

A new index was created on the table `public.prices`. The index was created non-concurrently, which blocks all writes to the table. Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.

##### Rewrote table or index while holding dangerous lock

ID: `E10`

A table or index was rewritten while holding a lock that blocks many operations. This blocks many operations on the table or index while the rewrite is in progress. A safer way is: Build a new table or index, write to both, then swap them.

The Table `public.prices` was rewritten while holding `AccessExclusiveLock` on the Table `public.authors`. This blocks `SELECT`, `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE`, `UPDATE`, `DELETE`, `INSERT`, `MERGE` while the rewrite is in progress.

