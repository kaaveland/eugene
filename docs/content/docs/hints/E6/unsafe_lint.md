---

title:  Linted matching transaction
weight: 40
---



## Eugene 🔒 lint report of `examples/E6/bad/1.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene). Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement or by passing --ignore E123 on the command line.

The migration script passed all the checks ✅

### Statement number 1

### SQL

```sql
create table authors (
    id integer generated always as identity primary key,
    name text not null
)
```

No checks matched for this statement. ✅


## Eugene 🔒 lint report of `examples/E6/bad/2.sql`

This is a human readable SQL script safety report generated by [eugene](https://github.com/kaaveland/eugene). Keep in mind that lints can be ignored by adding a `-- eugene: ignore E123` comment to the SQL statement or by passing --ignore E123 on the command line.

The migration script did not pass all the checks ❌

### Statement number 1

### SQL

```sql
set local lock_timeout = '2s'
```

No checks matched for this statement. ✅

### Statement number 2

### SQL

```sql
create index authors_name_idx on authors (name)
```

### Lints

##### Creating a new index on an existing table

ID: `E6`

A new index was created on an existing table without the `CONCURRENTLY` keyword. This blocks all writes to the table while the index is being created. A safer way is: Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`.

Statement takes `ShareLock` on `public.authors`, blocking writes while creating index `public.authors_name_idx`
