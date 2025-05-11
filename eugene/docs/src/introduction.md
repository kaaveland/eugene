# Introduction to eugene

Eugene helps you write zero downtime schema migration scripts for PostgreSQL databases.
Sometimes, the most straightforward way to make a change to your database schema is 
also quite risky, due to locking issues and lock queues. Eugene has two modes that can 
help you spot dangerous patterns and can suggest a safer way to achieve the same effect
in many cases, and it is easy to get started with both:

- `eugene lint` will perform syntax tree analysis of your SQL script using the PostgreSQL parser.
- `eugene trace` run your scripts in a temporary PostgreSQL server, and inspect locks.
- Both understand git and can easily check only what's new on your branch, commit or tag.
- Both handle folders with version named scripts, and run scripts in the right order.
- Easy to run in CI, to post markdown reports to your PRs.
<h2><label for="sql">Demo</label></h2>

Feel free to try out Eugene by playing around with the SQL script
in the text area below. When you click the "Check" button, Eugene
will analyze the scripts and let you know if it found any issues.

<div class="demo-area">
<form hx-post="https://api.kaveland.no/eugene/app/lint.html" 
      hx-target="#output"
      hx-on-htmx-response-error="check_for_413(event);">
<input type="hidden" name="sql" id="sql-input" value="">
<div id="sql" class="sql-playground">
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
  alter column title set not null;
alter table books
  alter column author set not null;
-- file: set_unique.sql
set local lock_timeout = '2s';
alter table books add constraint
  unique_title_author unique (title, author);
</div>
<script src="https://cdnjs.cloudflare.com/ajax/libs/ace/1.34.2/ace.js" integrity="sha512-WdJDvPkK4mLIW1kpkWRd7dFtAF6Z0xnfD3XbfrNsK2/f36vMNGt/44iqYQuliJZwCFw32CrxDRh2hpM2TJS1Ew==" crossorigin="anonymous" referrerpolicy="no-referrer"></script>
<script>
var editor = ace.edit("sql", {
  mode: "ace/mode/sql",
  selectionStyle: "text",
  minLines: 20,
  maxLines: 40,
});
editor.resize();
document.getElementById('sql-input').value = editor.getValue();
editor.session.on('change', function() {
  document.getElementById('sql-input').value = editor.getValue();
  document.getElementById('hx-errors').innerHTML = '';
});
function check_for_413(event) {
  var statusText = event.detail.xhr.statusText;
  if (event.detail.xhr.status === 413) {
    document.getElementById('hx-errors').innerHTML = 
      '<div class="warning"><p>' + statusText + '</p><p>The SQL script is too large. Please try a smaller script.</p></div>';  
  } else {
    document.getElementById('hx-errors').innerHTML = 
      '<div class="warning"><p>' + statusText + '</p><p>Unable to lint script.</p></div>';
  }
}
</script>
<div>
<button class="float-right button-cta" id="random-example">Another Example</button> 
<button class="float-right button-cta" id="submit">Check</button>
<script>
function fetch_new_example(event) {
    event.preventDefault();
    fetch('https://blog.kaveland.no/eugene/app/random.sql')
        .then(response => response.text())
        .then(data => {
              editor.setValue(data); 
              editor.clearSelection();
              document.getElementById('sql-input').value = data;
    });
}
document.getElementById("random-example").addEventListener("click", fetch_new_example);
</script>
</div>
</form>
<div id="hx-errors"></div>
<div id="output"></div>
</div>

The demo corresponds to using `eugene lint` on a folder of SQL scripts
on your local machine. You can also use `eugene trace` to run the scripts,
which can pick up more issues, some of which `eugene lint` can't detect.

## Installing eugene

You can use [mise](https://mise.jdx.dev/) with the `ubi` backend to install `eugene`:

```shell
mise use ubi:kaaveland/eugene@latest
```

You can also install `eugene` using cargo, but this requires you to have rust
and some other build tools installed. To install rust, you can use 
[rustup](https://rustup.rs/).

In addition to rust, you need:

- `gcc` and `g++` *or* `clang` and `clang++`
  + on macos, you get these with `xcode-select --install`
  + on ubuntu, install with `sudo apt install clang`
- `cmake`
  + on macos, you can get this with `brew install cmake`
  + on ubuntu, you can get this with `sudo apt install cmake`
    
After you have rust and the other build tools installed, you can install `eugene` with:

```sh
cargo install eugene
```


It is also available as a Docker image:

```sh
docker run --rm -v $(pwd):/workdir \
  ghcr.io/kaaveland/eugene:latest \
  lint /workdir
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

## Blog

I frequently blog about software development and other topics, here's
[blog posts about egene](https://kaaveland.github.io/tags/eugene/).

## Release notes

The [releases page](https://github.com/kaaveland/eugene/releases) is 
the best place to find release notes for `eugene`.
