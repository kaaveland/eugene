## Eugene 🔒 lint report of `examples/E2/good/1.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script passed all the checks ✅

### Statement number 1
#### SQL
```sql
-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text
)
```
No checks matched for this statement. ✅

## Eugene 🔒 lint report of `examples/E2/good/2.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script passed all the checks ✅

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
alter table authors
    add constraint check_name_not_null
        check (name is not null) not valid
```
No checks matched for this statement. ✅

## Eugene 🔒 lint report of `examples/E2/good/3.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script passed all the checks ✅

### Statement number 1
#### SQL
```sql
-- 3.sql
set local lock_timeout = '2s'
```
No checks matched for this statement. ✅
### Statement number 2
#### SQL
```sql
alter table authors
    validate constraint check_name_not_null
```
No checks matched for this statement. ✅

## Eugene 🔒 lint report of `examples/E2/good/4.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement
or by passing --ignore E123 on the command line.

The migration script passed all the checks ✅

### Statement number 1
#### SQL
```sql
-- 4.sql
set local lock_timeout = '2s'
```
No checks matched for this statement. ✅
### Statement number 2
#### SQL
```sql
-- eugene trace knows name has a valid not null check, but eugene lint doesn't
-- eugene: ignore E2
alter table authors
    alter name set not null
```
No checks matched for this statement. ✅
