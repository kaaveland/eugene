# Careful with That Lock, Eugene

![Tests](https://github.com/kaaveland/eugene/actions/workflows/run_tests.yml/badge.svg?branch=main)
![crates.io](https://img.shields.io/crates/v/eugene.svg)
![docs.rs](https://img.shields.io/docsrs/eugene)
![release](https://img.shields.io/github/release-date/kaaveland/eugene)
![GitHub License](https://img.shields.io/github/license/kaaveland/eugene)

`eugene` is a linter and command line tool for reviewing SQL migration scripts for postgres. 

There are useful examples of the kind of patterns eugene can help you pick up
available over at the [user documentation site](https://kaveland.no/eugene). This README
is primarily for development work on `eugene`, with only small convenience sections
for users.

## Installation

The tool documentation at [kaveland.no/eugene](https://kaveland.no/eugene) is the 
best place to get usage documentation, such as installation instructions and examples.

Some of those instructions are repeated here for convenience:

```bash
cargo install eugene
```

You can use the docker image `ghcr.io/kaaveland/eugene`. For example:

```shell
docker run --rm -it \
  -ePGPASS=postgres \
  -v./examples/:/examples \
  ghcr.io/kaaveland/eugene:latest \
  lint /examples/alter_column_not_null.sql
```

Releases are published to github as a binary with no dependencies, so you can
also download the binary from the [release page](https://github.com/kaaveland/eugene/releases)

The binary isn't signed and notarized, so on macos it'll give you a warning. If you
want to proceed anyway, you can use `xattr -d com.apple.quarantine eugene` to remove it.

To perform a local installation of your checkout out repository, you can use:

```bash
cargo install --path .
```

## Usage

`eugene` has a help command that should be fairly intuitive and can show you how to use the tool:

```bash
eugene help
```

Please refer to [kaveland.no/eugene/#usage](https://kaveland.no/eugene/#usage).

On the tool documentation site, there's output and help for each command,
and examples of how to use them. If you find the documentation site lacks
information you need, please open an issue or a pull request.

## Building the code

You can build the project with `cargo build` and run the tests with 
`cargo test`. The tests need to connect to a postgres database. The
easiest way to do this is to use the docker-compose setup at the root
of the repository:

```bash
docker-compose up -d
cargo test
```
## Tests

Unit tests go in the same file as the code they test. They are 
allowed to use a database connection, corresponding to the 
[docker-compose](https://github.com/kaaveland/eugene/blob/main/docker-compose.yml) setup 
or the [github workflow](https://github.com/kaaveland/eugene/blob/main/.github/workflows/run_tests.yml)
for the tests.

### Snapshot tests

Some tests generate output files in the repository, mainly under`/docs`. These 
are snapshot tests. If you see a change in the output of a generated file, you check
it in and commit it, if the change was intentional. If the change was unintentional,
you have to figure out why the output changed.

## Building documentation

The documentation relies on files that are generated by `cargo test`,
so you must first ensure that you can run the tests successfully,
refer to the previous section [Building the code](#building-the-code).

The tool documentation is built with `mdBooks` and is hosted on
[kaveland.no/eugene](https://kaveland.no/eugene). You can build it
using `mdbook serve docs`if you've already generated the
`SUMMARY.md` file with `cargo test`. **Note that direct changes to the
summary file will be overwritten by the tests, change the template
at `src/doc_summary.md.hbs` instead.**

The crate documentation is built using `cargo doc --open` -- the
open instruction is optional and will open the documentation in your
browser. The crate documentation is also hosted on 
[docs.rs](https://docs.rs/eugene/).

## Compatibility

`eugene` is tested with postgres versions `>= 12` on linux, and is
also tested on macos and windows for a narrower range of versions. 
It doesn't intentionally use any platform specific features or new 
features and should work with all of those. We should aim to be
compatible with all versions of postgres that are still supported 
by the postgres community, feel free to open an issue if you 
experience compatibility problems.

## Contributing

Contributions are welcome, but there's no roadmap for this project yet.
Feel free to open an issue, ideas and discussion are very welcome. If
you see an issue you'd like to fix, but don't know where to start, feel
free to ping @kaaveland to ask for help, or just to let him know you're
working on it.

## Releasing

To release a new version:
1. Update the version in `Cargo.toml`
2. Make sure to build so that `Cargo.lock` is updated
3. Commit the changes and push to the main branch
4. Tag the commit and push the tag
5. GitHub Workflows pick up the tag and build and release the new version to crates.io

## High level design

1. `src/bin/eugene.rs` should contain only code related to the command line interface 
   and standard in/err/out.
2. Structs that have public fields go somewhere in `eugene::output::output_format`, these
   are the structs we're OK with serializing to json or yaml, so we should consider them
   a contract of some sort.

### Lock tracing

The central idea is to run the SQL script statements in a transaction, and check what effects
they have on the state of the database:

- What locks are taken
- What changes are done tables, constraints, columns
- What indexes are created or dropped

The `tracing` module is responsible for storing this kind of state after running SQL statements
in a transaction.


### Linting

[pg_query.rs](https://github.com/pganalyze/pg_query.rs) breaks the script into statements and we convert
each statement into its syntax tree. These trees are pretty complex, because they can contain all possible
syntax in postgres, so they're converted to a more lightweight representation that fits better
for writing linting rules, see `src/linting/ast.rs`. Lint rules need a context, which is built gradually
from statements within each script, in addition to the lightweight syntax tree to work. This avoids some
false positives, by allowing the lints to skip checking statements that affect objects that can't be visible
to concurrent transactions. This means that `eugene lint` will not trigger on a `create index` statement
where the table was created in the same transaction.

## Credits

I have shamelessly stolen many migration patterns to detect from inspirational projects like
[strong_migrations](https://github.com/ankane/strong_migrations) and blog posts like
[PostgreSQL at Scale: Database Schema Changes Without Downtime](https://medium.com/paypal-tech/postgresql-at-scale-database-schema-changes-without-downtime-20d3749ed680).

## License

This project is licensed under the MIT License - see the [LICENSE.md](LICENSE.md) file for details.
