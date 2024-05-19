## Eugene 🔒 trace report of `examples/E1/bad/1.sql`

### Statement number 1 for 10 ms

### SQL

```sql
create table authors(id integer generated always as identity primary key, name text)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/E1/bad/2.sql`

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
alter table authors add constraint name_not_null check (name is not null)
```

#### Locks at start

No locks held at the start of this statement.

### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe |
|--------|--------|------|---------|-----|------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ |

### Hints

##### Validating table with a new constraint

ID: `E1`

A new constraint was added and it is already `VALID`. This blocks all table access until all rows are validated. A safer way is: Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later.

A new constraint `name_not_null` of type `CHECK` was added to the table `public.authors` as `VALID`. Constraints that are `NOT VALID` can be made `VALID` by `ALTER TABLE public.authors VALIDATE CONSTRAINT name_not_null` which takes a lesser lock.

