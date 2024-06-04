## Eugene 🔒 lint report of `examples/E1/bad/1.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script passed all the checks ✅

### Statement number 1
#### SQL
```sql
create table authors(
    id integer generated always as identity
        primary key,
    name text
)
```
No checks matched for this statement. ✅

## Eugene 🔒 lint report of `examples/E1/bad/2.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script did not pass all the checks ❌

### Statement number 1
#### SQL
```sql
set local lock_timeout = '2s'
```
No checks matched for this statement. ✅
### Statement number 2
#### SQL
```sql
alter table authors
    add constraint name_not_null
        check (name is not null)
```
#### Lints

##### [Validating table with a new constraint](https://kaveland.no/eugene/hints/E1/)

ID: `E1`

A new constraint was added and it is already `VALID`. This blocks all table access until all rows are validated. A safer way is: Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later.

Statement takes `AccessExclusiveLock` on `public.authors`, blocking reads until constraint `name_not_null` is validated.