# Eugene 🔒 trace report of `examples/E3/bad/1.sql`

## Statement number 1 for 26 ms

### SQL

```sql
create table authors (
    id integer generated always as identity primary key,
    name text not null,
    meta json
)
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


