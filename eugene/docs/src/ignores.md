# Ignoring rules

Both `eugene lint` and `eugene trace` can be instructed to ignore rules, so that
false positives can be suppressed, or warnings that aren't relevant for your use case
can be hidden.

You can ignore specific rule IDs for an entire transaction, using the command line flag:

```shell
eugene lint --ignore E2 my_script.sql
eugene trace --ignore E2 my_script.sql
```

You can ignore all rule IDs for a single statement:

```sql
-- eugene: ignore
alter table books alter column title set not null;
```

You can ignore specific rule IDs for a single statement:

```sql
-- eugene: ignore E2, E3
alter table books alter column title set not null;
```
