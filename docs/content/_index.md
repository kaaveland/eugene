+++
title = "Careful With that Lock, Eugene"
type = "docs"
+++


# Introduction to eugene

Eugene is a tool designed to help you write safe schema migrations for PostgreSQL. Sometimes,
the most straightforward way to make a change to your database schema is also quite risky,
due to locking issues and lock queues. Eugene has two modes that can help you spot these
dangerous patterns and can suggest a safer way to achieve the same effect in many cases.

## Installing eugene

You can install `eugene` using cargo:

```sh
cargo install eugene
```

It is also available as a Docker image:

```sh
docker run --rm -v $(pwd):/workdir \
  kaaveland/eugene:latest \ 
  lint /workdir/my_script.sql
```

Eugene is available as a binary for Linux and macOS. You can download the latest release from
the [releases page](https://github.com/kaaveland/eugene/releases). Note that the binaries
are not notarized and signed for macOS, so you may need to allow the binary to run by
removing its quarantine attribute:

```sh
xattr -d com.apple.quarantine eugene
```

## Source code and issue tracker

The source code is available on [GitHub](https://github.com/kaaveland/eugene/), where
it is also possible to report issues and suggest improvements.

`eugene` is licensed under the MIT license.

## Usage

Eugene has a number of subcommands, and can tell you about them:

```sh
eugene help
```

The two main subcommands are [`eugene lint`](/eugene/docs/lint)
and [`eugene trace`](/eugene/docs/trace), which both have their own page.
`eugene lint` will perform syntax tree analysis of your SQL script
using the PostgreSQL parser, while `eugene trace` will actually run
it in a transaction and inspect the effects of the script on the 
database. It will be easier to get started with `eugene lint` and
it can catch many dangerous patterns, but it may also report some 
false positives and might not pick up everything that `eugene trace`
can catch.

## Other commands

`eugene explain` can tell you about PostgreSQL locking modes, what
they're used for, and which operations in the database that may get
blocked from certain relation level locks.

`eugene hints` can tell you about the hints that Eugene can give 
you, what they mean, and in many cases, what you can do to avoid
the dangerous pattern. It will also tell you whether a hint is
supported by `eugene lint` or by `eugene trace` or by both.

## Hints provided by eugene

See [hints](/eugene/docs/hints/) for a list of hints that Eugene can give you.
