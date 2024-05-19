# eugene lint

The `eugene lint` command will analyze the syntax tree of a SQL script and spot a number
of dangerous patterns. It works by using the `pg_query` library to parse SQL scripts
using the same parser that PostgreSQL uses internally. This allows Eugene to work with
the same kind of syntax trees that the server uses.

`eugene lint` will attempt to keep track of whether objects are new in the same transaction,
so that it can avoid false positives for tables that aren't visible to other transactions yet.
In some cases, it can not help but report false positives, since it can't know the DDL of the
tables that are being referenced. For example, it can't know if a type change is safe. It
is easy to ignore these false positives by adding a comment to the SQL script, see
[ignores](/eugene/docs/ignores).

`eugene lint` can catch many things that it is specifically designed to catch, but it must
have reasonably precise rules. Some SQL statements will implicitly create indexes, which
will prevent writes to the table, and `eugene lint` will catch those that it knows about,
but there may be some ways for this to happen that it doesn't know about.

## Usage

```shell
$ eugene help lint
{{#include shell_output/lint }}
```

