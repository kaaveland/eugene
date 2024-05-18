# Eugene 🔒 trace report of `examples/W12/good/1.sql`

## Statement number 1 for 10 ms

### SQL

```sql
create table authors(id integer generated always as identity primary key, name text, email text)
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



# Eugene 🔒 trace report of `examples/W12/good/2.sql`

## Statement number 1 for 10 ms

### SQL

```sql
set lock_timeout = '2s'
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


## Statement number 2 for 10 ms

### SQL

```sql
-- eugene: ignore E2
alter table authors
  alter column name set not null,
  alter column email set not null
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |

