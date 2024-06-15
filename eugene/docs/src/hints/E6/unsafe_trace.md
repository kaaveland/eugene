## Eugene 🔒 trace report of `examples/E6/bad/1.sql`



### Statement number 1 for 10ms

#### SQL

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



## Eugene 🔒 trace report of `examples/E6/bad/2.sql`



### Statement number 1 for 10ms

#### SQL

```sql
-- 2.sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



### Statement number 2 for 10ms

#### SQL

```sql
create index
    authors_name_idx on authors (name)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `authors` | `ShareLock` | Table | 1 | ❌ | 10 |

#### Hints

##### [Creating a new index on an existing table](https://kaveland.no/eugene/hints/E6/)
ID: `E6`

A new index was created on an existing table without the `CONCURRENTLY` keyword. This blocks all writes to the table while the index is being created. A safer way is: Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`.

A new index was created on the table `public.authors`. The index `public.authors_name_idx` was created non-concurrently, which blocks all writes to the table. Use `CREATE INDEX CONCURRENTLY` to avoid blocking writes.

