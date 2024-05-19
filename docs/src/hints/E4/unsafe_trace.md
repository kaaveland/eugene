## Eugene 🔒 trace report of `examples/E4/bad/1.sql`

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



## Eugene 🔒 trace report of `examples/E4/bad/2.sql`

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

### Statement number 3 for 10 ms

### SQL

```sql
select count(*) from authors
```

#### Locks at start

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |

### New locks taken

No new locks taken by this statement.


### Hints

##### Running more statements after taking `AccessExclusiveLock`

ID: `E4`

A transaction that holds an `AccessExclusiveLock` started a new statement. This blocks all access to the table for the duration of this statement. A safer way is: Run this statement in a new transaction.

The statement is running while holding an `AccessExclusiveLock` on the Table `public.authors`, blocking all other transactions from accessing it.

