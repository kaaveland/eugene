## ✅ Eugene trace report

Script name: `examples/W13/good/1.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 1.sql
create table document_type(
    type_name text primary key
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ✅ Statement number 2 for 10ms

```sql
insert into document_type
  values('invoice'), ('receipt'), ('other')
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ✅ Statement number 3 for 10ms

```sql
create table document (
    id int generated always as identity
        primary key,
    type text
        references document_type(type_name)
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.

