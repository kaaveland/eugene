---

title:  Traced safer transaction
weight: 60
---


## Eugene 🔒 trace report of `examples/W13/good/1.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create table document_type(type_name text primary key)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


### Statement number 2 for 10 ms

### SQL

```sql
insert into document_type values('invoice'), ('receipt'), ('other')
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


### Statement number 3 for 10 ms

### SQL

```sql
create table document (id int generated always as identity primary key, type text references document_type(type_name))
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.


