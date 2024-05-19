## Eugene 🔒 trace report of `examples/E7/good/1.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create table authors(
    id integer generated always as identity primary key,
    name text not null
)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E7/good/2.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create unique index concurrently authors_name_unique on authors(name)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E7/good/3.sql`

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
alter table authors add constraint unique_name unique using index authors_name_unique
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |

