## ✅ Eugene lint report

Script name: `examples/E1/good/1.sql`

This is a human readable SQL lint report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lint rules can be ignored in the following two ways:

  1. By appending comment directives like `-- eugene: ignore E123` to the SQL statement.
  2. By passing `--ignore E123` on the command line.

### ✅ Statement number 1

```sql
-- 1.sql
create table authors(
    id integer generated always as identity
        primary key,
    name text
)
```

## ✅ Eugene lint report

Script name: `examples/E1/good/2.sql`

This is a human readable SQL lint report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lint rules can be ignored in the following two ways:

  1. By appending comment directives like `-- eugene: ignore E123` to the SQL statement.
  2. By passing `--ignore E123` on the command line.

### ✅ Statement number 1

```sql
-- 2.sql
set local lock_timeout = '2s'
```

### ✅ Statement number 2

```sql
alter table authors
    add constraint name_not_null
        check (name is not null) not valid
```

## ✅ Eugene lint report

Script name: `examples/E1/good/3.sql`

This is a human readable SQL lint report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lint rules can be ignored in the following two ways:

  1. By appending comment directives like `-- eugene: ignore E123` to the SQL statement.
  2. By passing `--ignore E123` on the command line.

### ✅ Statement number 1

```sql
-- 3.sql
set local lock_timeout = '2s'
```

### ✅ Statement number 2

```sql
alter table authors
    validate constraint name_not_null
```
