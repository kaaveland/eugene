## Eugene 🔒 trace report of `examples/E9/bad/1.sql`

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



## Eugene 🔒 trace report of `examples/E9/bad/2.sql`

### Statement number 1 for 10 ms

### SQL

```sql
alter table authors add column email text
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |

### Hints

##### Taking dangerous lock without timeout

ID: `E9`

A lock that would block many common operations was taken without a timeout. This can block all other operations on the table indefinitely if any other transaction holds a conflicting lock while `idle in transaction` or `active`. A safer way is: Run `SET LOCAL lock_timeout = '2s';` before the statement and retry the migration if necessary.

The statement took `AccessExclusiveLock` on the Table `public.authors` without a timeout. It blocks `SELECT`, `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE`, `UPDATE`, `DELETE`, `INSERT`, `MERGE` while waiting to acquire the lock.

