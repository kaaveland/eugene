## ✅ Eugene trace report

Script name: `examples/W14/good/1.sql`


### ✅ Statement number 1 for 10ms

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


## ✅ Eugene trace report

Script name: `examples/W14/good/2.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 2.sql
create unique index concurrently
    authors_name_key on authors(name)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


## ✅ Eugene trace report

Script name: `examples/W14/good/3.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 3.sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ✅ Statement number 2 for 10ms

```sql
-- eugene: ignore E2
-- This is a demo of W14, so we can ignore E2 instead of the
-- multi-step migration to make the column NOT NULL safely
alter table authors
    alter column name set not null
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |


## ❌ Eugene trace report

Script name: `examples/W14/good/4.sql`


### ❌ Statement number 1 for 10ms

```sql
-- 4.sql
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

#### Triggered rules

##### `E1`: [Validating table with a new constraint](https://kaveland.no/eugene/hints/E1/)

A new constraint `authors_name_pkey` of type `PRIMARY KEY` was added to the table `public.authors` as `VALID`. Constraints that are `NOT VALID` can be made `VALID` by `ALTER TABLE public.authors VALIDATE CONSTRAINT authors_name_pkey` which takes a lesser lock.

##### `E9`: [Taking dangerous lock without timeout](https://kaveland.no/eugene/hints/E9/)

The statement took `AccessExclusiveLock` on the Table `public.authors` without a timeout. It blocks `SELECT`, `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE`, `UPDATE`, `DELETE`, `INSERT`, `MERGE` while waiting to acquire the lock.
