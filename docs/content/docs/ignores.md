+++
title = "ignores"
weight = 70
+++

# Ignoring lints

Fortunately, it is easy for you to tell eugene to ignore rules for statements where it is wrong
by adding a special comment to your SQL script, like this:

```sql
-- eugene: ignore
alter table books alter column title set not null;
```

You can also ignore specific hints and warnings by specifying their reported id:

```sql
-- eugene: ignore: E2
alter table books alter column title set not null;
```

You can also ignore all lints for a statement:

```sql
-- eugene: ignore
alter table books alter column title set not null;
```

Or you can ignore specific lint IDs for an entire transaction, using the command line flag:

```shell
eugene lint --ignore E2 my_script.sql
# also works with tracing
eugene trace --ignore E2 my_script.sql
```

Both `eugene lint` and `eugene trace` will respect these comments and flags and will
not report failure if only ignored lints match.
