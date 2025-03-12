## ✅ Eugene trace report

Script name: `examples/E4/bad/1.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null
)
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


## ❌ Eugene trace report

Script name: `examples/E4/bad/2.sql`


### ✅ Statement number 1 for 10ms

```sql
-- 2.sql
set local lock_timeout = '2s'
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

No new locks taken by this statement.


### ✅ Statement number 2 for 10ms

```sql
alter table authors
    add column email text not null
```

#### Locks at start

No locks held at the start of this statement.

#### New locks taken

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |


### ❌ Statement number 3 for 10ms

```sql
select count(*) from authors
```

#### Locks at start

| Schema | Object | Mode | Relkind | OID | Safe | Duration held (ms) |
|--------|--------|------|---------|-----|------|--------------------|
| `public` | `authors` | `AccessExclusiveLock` | Table | 1 | ❌ | 10 |

#### New locks taken

No new locks taken by this statement.

#### Triggered rules

##### `E4`: [Running more statements after taking `AccessExclusiveLock`](https://kaveland.no/eugene/hints/E4/)

The statement is running while holding an `AccessExclusiveLock` on the Table `public.authors`, blocking all other transactions from accessing it.
