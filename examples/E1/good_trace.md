# Eugene 🔒 trace report of `examples/E1/good/1.sql`

## Statement number 1 for 26 ms

### SQL

```sql
create table authors(id integer generated always as identity primary key, name text)
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



# Eugene 🔒 trace report of `examples/E1/good/2.sql`

## Statement number 1 for 4 ms

### SQL

```sql
set local lock_timeout = '2s'
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


## Statement number 2 for 5 ms

### SQL

```sql
alter table authors add constraint name_not_null check (name is not null) not valid
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 24605 | ❌ |


# Eugene 🔒 trace report of `examples/E1/good/3.sql`

## Statement number 1 for 2 ms

### SQL

```sql
set local lock_timeout = '2s'
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


## Statement number 2 for 2 ms

### SQL

```sql
alter table authors validate constraint name_not_null
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


