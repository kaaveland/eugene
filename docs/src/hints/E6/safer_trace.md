## Eugene 🔒 trace report of `examples/E6/good/1.sql`



### Statement number 1 for 10ms

#### SQL

```sql
create table authors (
    id integer generated always as identity primary key,
    name text not null
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E6/good/2.sql`



### Statement number 1 for 10ms

#### SQL

```sql
create index concurrently authors_name_idx on authors (name)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


