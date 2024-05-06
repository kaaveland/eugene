# Careful with That Lock, Eugene

![Tests](https://github.com/kaaveland/eugene/actions/workflows/run_tests.yml/badge.svg?branch=main)
![crates.io](https://img.shields.io/crates/v/eugene.svg)
![docs.rs](https://img.shields.io/docsrs/eugene)
![release](https://img.shields.io/github/release-date/kaaveland/eugene)
![GitHub License](https://img.shields.io/github/license/kaaveland/eugene)

`eugene` is a proof of concept command line tool for reviewing locks taken by SQL
migration scripts in postgres. 

It is currently an experiment inspired by the observation that postgres has
transactional DDL, and therefore it is possible to inspect the locks held by the
current transaction in the `pg_locks` view. For more information about the goals of
this experiment, take a look at
[the blog post](https://kaveland.no/careful-with-that-lock-eugene.html) that started it.

## Installation

You can install `eugene` from [crates.io](https://crates.io/crates/eugene) using `cargo` from
[rustup](https://rustup.rs/):

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
docker run --rm -it \
  -ePGPASS=postgres \
  -v./examples/add_authors.sql:/add_authors.sql \
  ghcr.io/kaaveland/eugene:0.1.2 \
  trace --format markdown \
  --host pg-test --database test-db \
  /add_authors.sql
```

## Viewing migration hints

`eugene` knows about some common migration patterns that can cause problems with locks and in many cases,
it can suggest workarounds. To see what hints are available, run:

```bash
eugene hints
```

I have shamelessly stolen many such hints from inspirational projects like
[strong_migrations](https://github.com/ankane/strong_migrations) and blog posts like 
[PostgreSQL at Scale: Database Schema Changes Without Downtime](https://medium.com/paypal-tech/postgresql-at-scale-database-schema-changes-without-downtime-20d3749ed680).

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
ideas and discussion are very welcome. If you see an issue you'd like to fix, but don't know
where to start, feel free to ping @kaaveland to ask for help, or just to let him know you're
working on it.

## Building

You can build the project with `cargo build` and run the tests with `cargo test`. The tests
need to connect to a postgres database. The easiest way to do this is to use the docker-compose
setup at the root of the repository:

```bash
docker-compose up -d
cargo test
```

## Documentation

You can browse this locally with:

```bash
cargo doc --open
```

Docs are also hosted at [docs.rs](https://docs.rs/eugene/).

## Releasing

To release a new version:
1. Update the version in `Cargo.toml`
2. Make sure to build so that `Cargo.lock` is updated
3. Commit the changes and push to the main branch
4. Tag the commit and push the tag
5. GitHub Workflows pick up the tag and build and release the new version to crates.io


## High level design

The central idea is to run the SQL script statements in a transaction, and check what effects
they have on the state of the database:
- What locks are taken
- What changes are done tables, constraints, columns
- What indexes are created or dropped

The `tracing` module is responsible for storing this kind of state after running SQL statements
in a transaction. Other principles are:

1. `src/bin/eugene.rs` should contain only code related to the command line interface and standard in/err/out.
2. Structs that are serializable go in `output` 
3. Structs that have public fields go somewhere in `output::output_format`
4. We prefer not to expose public fields of anything in `tracing`
5. That means we need to map from `tracing` to `output` to serialize output or expose fields
   - We `.clone()` liberally for this purpose, because eventually we'd like make the structs `Deserialize`.

## Tests

Tests are welcome and come in two flavors:

1. Unit tests go in the same file as the code they test. They are allowed to use a database connection, corresponding
   to the [docker-compose](https://github.com/kaaveland/eugene/blob/main/docker-compose.yml) setup or the 
   [github workflow](https://github.com/kaaveland/eugene/blob/main/.github/workflows/run_tests.yml) for the tests  
2. Integration tests go in the `tests` directory. These can only access public interfaces and therefore would the
   the right place to gauge how dependents would see the tool. In particular, we take snapshots of markdown reports
   that go in the examples directory, which we can use to track changes in the output format.

## Migration tool

`eugene` is not a migration tool like flyway or liquibase, and isn't intended to be one. There are
many excellent migration tools already, and the scope of `eugene` is only to help develop migrations
that are less likely to cause problems in a database that is in use by application code.

# License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
