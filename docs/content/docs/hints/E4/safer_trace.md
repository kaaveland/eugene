---

title:  Traced safer transaction
weight: 60
---


## Eugene 🔒 trace report of `examples/E4/good/1.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create table authors (
    id integer generated always as identity primary key,
    name text not null
)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E4/good/2.sql`

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
alter table authors add column email text not null
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |


## Eugene 🔒 trace report of `examples/E4/good/3.sql`

### Statement number 1 for 10 ms

### SQL

```sql
select count(*) from authors
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


