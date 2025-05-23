# eugene trace

The `eugene trace` command will actually execute your SQL script in a transaction. If you have
PostgreSQL installed, `eugene trace` can set up a temporary database server for you, and run
through all the SQL scripts you give it to trace them. If you prefer to use your own database
server, you can give provide `eugene trace` with connection information and it will roll back
scripts by default, in this case.

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
adding a comment to your SQL script, see [ignores](ignores.md).

If you want to run `eugene trace` in CI, or as a pre-commit hook, you can use `--git-diff=main`
or `-gmain` to trace files that are new/unstaged, or have changes in them since `main`. 
`eugene trace` will still run all the scripts, but will only check the ones that have changed.

## Usage

```shell
$ eugene help trace
{{#include shell_output/trace }}
```
