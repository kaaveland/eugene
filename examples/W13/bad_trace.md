# Eugene 🔒 trace report of `examples/W13/bad/1.sql`

## Statement number 1 for 10 ms

### SQL

```sql
create type document_type as enum ('invoice', 'receipt', 'other')
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


## Statement number 2 for 10 ms

### SQL

```sql
create table document (id int generated always as identity primary key, type document_type)
```

### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


