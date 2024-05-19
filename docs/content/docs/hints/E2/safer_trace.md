---

title:  Traced safer transaction
weight: 60
---


## Eugene 🔒 trace report of `examples/E2/good/1.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create table authors(
    id integer generated always as identity primary key,
    name text
)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E2/good/2.sql`

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
alter table authors add constraint check_name_not_null check (name is not null) not valid
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |


## Eugene 🔒 trace report of `examples/E2/good/3.sql`

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
alter table authors validate constraint check_name_not_null
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E2/good/4.sql`

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
-- eugene trace knows name has a valid not null check, but eugene lint doesn't
-- eugene: ignore E2
alter table authors alter name set not null
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |
