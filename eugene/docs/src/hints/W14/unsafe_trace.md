## Eugene 🔒 trace report of `examples/W14/bad/1.sql`



### Statement number 1 for 10ms

#### SQL

```sql
-- 1.sql
create table authors(
    name text
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/W14/bad/2.sql`



### Statement number 1 for 10ms

#### SQL

```sql
-- 2.sql
create unique index concurrently
    authors_name_key on authors(name)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



## Eugene 🔒 trace report of `examples/W14/bad/3.sql`



### Statement number 1 for 10ms

#### SQL

```sql
-- 3.sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.



### Statement number 2 for 10ms

#### SQL

```sql
alter table authors
    add constraint authors_name_pkey
        primary key using index authors_name_key
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |

#### Hints

##### [Validating table with a new constraint](https://kaveland.no/eugene/hints/E1/)
ID: `E1`

A new constraint was added and it is already `VALID`. This blocks all table access until all rows are validated. A safer way is: Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later.

A new constraint `authors_name_pkey` of type `PRIMARY KEY` was added to the table `public.authors` as `VALID`. Constraints that are `NOT VALID` can be made `VALID` by `ALTER TABLE public.authors VALIDATE CONSTRAINT authors_name_pkey` which takes a lesser lock.
##### [Validating table with a new `NOT NULL` column](https://kaveland.no/eugene/hints/E2/)
ID: `E2`

A column was changed from `NULL` to `NOT NULL`. This blocks all table access until all rows are validated. A safer way is: Add a `CHECK` constraint as `NOT VALID`, validate it later, then make the column `NOT NULL`.

The column `name` in the table `public.authors` was changed to `NOT NULL`. If there is a `CHECK (name IS NOT NULL)` constraint on `public.authors`, this is safe. Splitting this kind of change into 3 steps can make it safe:

1. Add a `CHECK (name IS NOT NULL) NOT VALID;` constraint on `public.authors`.
2. Validate the constraint in a later transaction, with `ALTER TABLE public.authors VALIDATE CONSTRAINT ...`.
3. Make the column `NOT NULL`


