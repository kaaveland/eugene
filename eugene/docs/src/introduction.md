# Introduction to eugene

Eugene is a tool designed to help you write safe schema migrations for PostgreSQL. Sometimes,
the most straightforward way to make a change to your database schema is also quite risky,
due to locking issues and lock queues. Eugene has two modes that can help you spot these
dangerous patterns and can suggest a safer way to achieve the same effect in many cases.

<h2><label for="sql">Demo</label></h2>

Feel free to try out Eugene by playing around with the SQL script
in the text area below. When you click the "Check" button, Eugene 
will analyze the scripts and let you know if it found any issues.

<div class="demo-area">
<form 
  hx-post="/eugene/app/lint.html"
  hx-target="#output">
<textarea id="sql" name="sql" rows="20" class="full-width">
-- You can use file markers like this to break migrations
-- into steps and run them in order.
-- file: create_table.sql
create table books (
    id serial primary key,
    title text,
    author text,
    published date
);
-- file: alter_table.sql
alter table books
  alter column text set not null;
alter table books
  alter column author set not null;
-- file: set_unique.sql
set local lock_timeout = '2s';
alter table books add constraint 
  unique_title_author unique (title, author);
</textarea>
<div>
<button class="float-right button-cta" id="submit">Check</button>
</div>
</form>

<div id="output"></div>
</div>

The demo corresponds to using `eugene lint` on a folder of SQL scripts
on your local machine. You can also use `eugene trace` to run the scripts,
which can pick up more issues, some of which `eugene lint` can't detect.

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

```shell
$ eugene help
{{#include shell_output/help}}
```

The two main subcommands are [`eugene lint`](./lint.md)
and [`eugene trace`](./trace.md), which both have their own page.
`eugene lint` will perform syntax tree analysis of your SQL script
using the PostgreSQL parser, while `eugene trace` will actually run
it in a transaction and inspect the effects of the script on the 
database. It will be easier to get started with `eugene lint` and
it can catch many dangerous patterns, but it may also report some 
false positives and might not pick up everything that `eugene trace`
can catch.


## Hints provided by eugene

See [hints](./hints.md) for a list of hints that Eugene can give you.
