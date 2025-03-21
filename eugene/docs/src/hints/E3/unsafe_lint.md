## ❌ Eugene lint report

Script name: `examples/E3/bad/1.sql`

This is a human readable SQL lint report generated by [eugene](https://github.com/kaaveland/eugene).
Keep in mind that lint rules can be ignored in the following two ways:

  1. By appending comment directives like `-- eugene: ignore E123` to the SQL statement.
  2. By passing `--ignore E123` on the command line.

### ❌ Statement number 1

```sql
-- 1.sql
create table authors (
    id integer generated always as identity
        primary key,
    name text not null,
    meta json
)
```

#### Triggered rules

##### `E3`: [Add a new JSON column](https://kaveland.no/eugene/hints/E3/)

Created column `meta` with type `json`. The `json` type does not support equality and should not be used, use `jsonb` instead.
