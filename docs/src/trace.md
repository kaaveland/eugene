# eugene trace

The `eugene trace` command will actually execute your SQL script in a transaction. By default,
it will roll back the transaction once it is done inspecting the metadata of the database.

PostgreSQL has a number of interesting features that can help `eugene trace` look for dangerous
patterns while executing SQL statements in a transaction. `eugene trace` will look at the data
types of every column, it will discover new indexes and constraints, and it will discover when
database objects get moved to a new location on disk -- that is, table or index rewrites.

Since `eugene trace` has so much information, it is much less likely to trigger false positives
than `eugene lint`, but it is also slower and requires a live database connection.

`eugene trace` can catch broad categories of dangerous patterns -- sometimes, it will discover
a table rewrite that `eugene lint` can not detect, but it may not be able to tell you about
why that table rewrite happened. `eugene trace` will discover all indexes and constraints
created in a transaction, even if they were implicitly created. If you need to tell 
`eugene trace` that you know a statement to be safe, you can tell it to ignore a lint by
adding a comment to your SQL script, see [ignores](/eugene/docs/ignores).

## Usage

```shell
$ eugene help trace
{{#include shell_output/trace }}
```
