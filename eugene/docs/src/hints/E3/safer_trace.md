## Eugene 🔒 trace report of `examples/E3/good/1.sql`



### Statement number 1 for 10ms

#### SQL

```sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null,
    meta jsonb
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


