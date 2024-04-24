# Careful with That Lock, Eugene

`eugene` is a proof of concept command line tool for reviewing locks taken by SQL
migration scripts in postgres. 

It is currently an experiment inspired by the 
observation that postgres has transactional DDL, and therefore it is possible to
inspect the locks held by the current transaction in the `pg_locks` view. For more
information about the goals of this experiment, take a look at 
[the blog post](https://kaveland.no/careful-with-that-lock-eugene.html) that started it.

## Installation

Currently, the only way to install `eugene` is to clone the repository and build it. You will
need `cargo` installed to do this, see [rustup](https://rustup.rs/).

```bash
git clone git@github.com:kaaveland/eugene.git
cd eugene
cargo build --release
```

## Usage

`eugene` has a help command that should be up-to-date with the tools current capabilites.

```bash
eugene help
```

## Explaining lock modes

`eugene` knows about the lock modes in postgres, for example `eugene explain ShareLock` will emit:

```
Lock mode: ShareLock
Used for: CREATE INDEX
Conflicts with: RowExclusiveLock, ShareUpdateExclusiveLock, ShareRowExclusiveLock, ExclusiveLock, AccessExclusiveLock
Blocked query types: UPDATE, DELETE, INSERT, MERGE
Blocked DDL operations: VACUUM, ANALYZE, CREATE INDEX CONCURRENTLY, CREATE STATISTICS, REINDEX CONCURRENTLY, ALTER INDEX, ALTER TABLE, CREATE TRIGGER, ALTER TABLE, REFRESH MATERIALIZED VIEW CONCURRENTLY, ALTER TABLE, DROP TABLE, TRUNCATE, REINDEX, CLUSTER, VACUUM FULL, REFRESH MATERIALIZED VIEW
```

Use `eugene lock-modes` or refer to [the postgres documentation](https://www.postgresql.org/docs/current/explicit-locking.html) 
to learn more about lock modes.

## Tracing locks taken by a transaction

To review a SQL script for locks, you will need to run `eugene trace` and provide some
connection information to a database. For example, for the local docker-compose setup:

```bash
# Currently the only way to provide eugene with a password is through the PGPASS environment variable
export PGPASS=postgres 
createdb --host localhost -U postgres --port 5432 example-db
echo 'create table books(id serial primary key, title text);' |
  psql --host localhost -U postgres --port 5432 example-db
echo 'create table author(name text not null); alter table books alter column title set not null;' |
  eugene trace --host localhost -U postgres --port 5432 --database example-db -- -  
```

You should see some output that looks like this:

```
# Statement 1: SQL: create table author(name text not null);
# New locks taken: None
# Duration: 5.686041ms
Statement 2: SQL: alter table books alter column title set not null;
New locks taken:
  - AccessExclusiveLock on Table public.books blocks SELECT, FOR UPDATE, FOR NO KEY UPDATE, FOR SHARE, FOR KEY SHARE, UPDATE, DELETE, INSERT, MERGE
Duration: 2.178208ms
```

Note that `eugene` only logs locks that target relations visible to other transactions, so it does 
log any lock for the `author` table in this instance. By default, `eugene trace` will roll back 
transactions, and you should pass `-c` or `--commit` if this is not what you want.


### Complex SQL scripts and variables

`eugene trace` supports simple placeholders in the SQL script, so that statements like 
`set role ${dba};` can be used by providing `--var dba=postgres` on the command line. Any
number of vars may be provided by repeating the option.

Note that some SQL scripts contain syntax that breaks `eugene` at the moment, for instance
`$body$` is not supported and things like comment syntax inside strings may cause problems,
since the parser is very simple. This will be addressed in future versions, if the tool turns
out to be useful.

### Output format

Note that the output format is subject to change, in the near future `eugene` will be able to
output json or markdown or something else that's suitable for use in CI/CD pipelines.

# Combatibility

`eugene` should work with most versions of postgres after 12, it isn't running any 
particularly fancy queries or using any new features or types. If you find that 
it doesn't work with your version of postgres, feel free to open an issue.

# Contributing

Contributions are welcome, but there's no roadmap for this project yet. Feel free to open an issue,
ideas and discussion are very welcome.

# Future work

These are some goals for the future of `eugene`:

- Support more output formats (globally, for all commands):
    + JSON, to let people build their own tools or CI/CD rules on top of `eugene`
    + Markdown and HTML, to make `eugene` more useful in CI/CD pipelines, such as by posting comments on pull requests
- Automatically detect table rewrites so that `eugene` can report on locks that potentially held for a long time
- Automatically check for queries that may get blocked in `pg_stat_statements`
    + In a CI situation, this could be used to prevent merges that would cause downtime assuming most queries run 
      as part of the CI pipeline.
- Add output filtering, such as emitting only locks that certain kinds of queries
- Exit codes that reflect the presence of dangerous locks, suitable for use in CI
- Support more complex SQL scripts
- Pick up certain dangerous patterns automatically and automatically suggest better ways to achieve the same result
  + Inspired by [strong_migrations](https://github.com/ankane/strong_migrations) which has a similar goal for Rails migrations.
- Investigate whether we could do some good by creating an extension with [pgrx](https://github.com/pgcentralfoundation/pgrx).
- Trace locks across object renames, eg. if `book` is locked and then renamed to `books`, `eugene` should be able to 
  understand that the `books` lock is the same as the `book` lock and not output a new lock.

# License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
