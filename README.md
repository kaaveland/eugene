# Careful with That Lock, Eugene

![Tests](https://github.com/kaaveland/eugene/actions/workflows/run_tests.yml/badge.svg?branch=main)
![crates.io](https://github.com/kaaveland/eugene/actions/workflows/release_to_crates_io_from_main.yml/badge.svg?branch=main)

`eugene` is a proof of concept command line tool for reviewing locks taken by SQL
migration scripts in postgres. 

It is currently an experiment inspired by the 
observation that postgres has transactional DDL, and therefore it is possible to
inspect the locks held by the current transaction in the `pg_locks` view. For more
information about the goals of this experiment, take a look at 
[the blog post](https://kaveland.no/careful-with-that-lock-eugene.html) that started it.

## Installation

You can install `eugene` from [crates.io](https://crates.io/crates/eugene) using `cargo` from [rustup](https://rustup.rs/):

```bash
cargo install eugene --bin
```

To perform a local installation of your checkout out repository, you can use:

```bash
cargo install --path . --bin
```

## Usage

`eugene` has a help command that should be fairly intuitive and can show you how to use the tool:

```bash
eugene help
```

## Docker images

You can use the docker image `ghcr.io/kaaveland/eugene` to run the tool. For example:

```shell
docker run --rm -it ghcr.io/kaaveland/eugene:latest \
  trace --format markdown \
  --host pg-test --database test-db \
  examples/add_authors.sql
```
## Explaining lock modes

`eugene` knows about the lock modes in postgres, and can explain them to you:

```bash
eugene modes
```

Or

```
eugene explain AccessExclusive
```

Use `eugene modes` or refer to [the postgres documentation](https://www.postgresql.org/docs/current/explicit-locking.html) 
to learn more about lock modes.

## Lock tracing reports

`eugene` can produce reports in a verbose markdown that is suitable for human reading. Take a look
at [the examples](https://github.com/kaaveland/eugene/tree/main/examples) to see how the output looks.

`eugene` can also produce a json output that is more suitable for machine processing.

To review a SQL script for locks, you will need to run `eugene trace` and provide some
connection information to a database. For example, for the local docker-compose setup:

```bash
# You can use ~/.pgpass for the password, or set it in the environment
export PGPASS=postgres 
docker compose up -d
sleep 5 # wait for the database to start
eugene trace --database example-db \
  --format json \ # or markdown
  examples/add_authors.sql
```

You should see some output that looks like this, but with a lot more details:

```json
{
  "name": "add_authors.sql",
  "start_time": "2024-05-05T21:27:09.739410+02:00",
  "total_duration_millis": 1015,
  "all_locks_acquired": []
}
```

Note that `eugene` only logs locks that target relations visible to other transactions, so it does 
not log any lock for the `author` table in this instance. By default, `eugene trace` will roll back 
transactions, and you should pass `-c` or `--commit` if this is not what you want.

### Complex SQL scripts and variables

`eugene trace` supports simple placeholders in the SQL script, so that statements like 
`set role ${dba};` can be used by providing `--var dba=postgres` on the command line. Any
number of vars may be provided by repeating the option.

Note that some SQL scripts contain syntax that breaks `eugene` at the moment, for instance
`$body$` is not supported and things like comment syntax inside strings may cause problems,
since the parser is very simple. This will be addressed in future versions, if the tool turns
out to be useful.

### Compatibility

`eugene` is tested with postgres versions `>= 12` on linux, and is also tested on macos
and windows for a narrower range of versions. It doesn't intentionally use any platform
specific features or new features and should work with all of those. We build images
for linux on debian:slim with the gnu toolchain.

# Contributing

Contributions are welcome, but there's no roadmap for this project yet. Feel free to open an issue,
ideas and discussion are very welcome. 

## Migration tool

`eugene` is not a migration tool like flyway or liquibase, and isn't intended to be one. There are
many excellent migration tools already, and the scope of `eugene` is only to help develop migrations
that are less likely to cause problems in a database that is in use by application code.

# License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
