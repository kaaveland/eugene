## Eugene 🔒 lint report of `examples/E5/bad/1.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script passed all the checks ✅

### Statement number 1
#### SQL
```sql
-- 1.sql
create table prices (
    id integer generated always as identity
        primary key,
    price int not null
)
```
No checks matched for this statement. ✅

## Eugene 🔒 lint report of `examples/E5/bad/2.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script did not pass all the checks ❌

### Statement number 1
#### SQL
```sql
-- 2.sql
set local lock_timeout = '2s'
```
No checks matched for this statement. ✅
### Statement number 2
#### SQL
```sql
alter table prices
    alter price set data type bigint
```
#### Lints

##### [Type change requiring table rewrite](https://kaveland.no/eugene/hints/E5/)

ID: `E5`

A column was changed to a data type that isn't binary compatible. This causes a full table rewrite while holding a lock that prevents all other use of the table. A safer way is: Add a new column, update it in batches, and drop the old column.

Changed type of column `price` to `pg_catalog.int8` in `.prices`. This operation requires a full table rewrite with `AccessExclusiveLock` if `pg_catalog.int8` is not binary compatible with the previous type of `price`. Prefer adding a new column with the new type, then dropping/renaming..
