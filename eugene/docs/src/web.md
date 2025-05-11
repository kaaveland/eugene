# eugene web

Eugene has a tiny web API that can be used to lint SQL scripts. This API exposes
some functionality for demo purposes only. It's running with very limited 
resources, so please be kind to it.

It is written using [axum](https://crates.io/crates/axum), mostly to learn 
something about how to write web APIs in Rust. You can check out the code
in the [eugene-web](https://github.com/kaaveland/eugene/tree/main/eugene-web)
crate in the eugene repository.

## Endpoints

All the endpoints are relative to `https://api.kaveland.no/eugene/app` or
`http://localhost:3000/eugene/app` if you're running it locally.

### `POST /lint.html`

This endpoint accepts a form with a parameter named `sql` that contains
one or more SQL scripts preceeded by a `-- name.sql` marker. Each script
can contain multiple statements. The API returns html. This endpoint is
used by the form in the [introduction](introduction.md) page.

### `POST /lint.raw`

This endpoint accepts a SQL script in the body and responds with a plain text
response. It's suitable for use with `curl` from the terminal, you can check a
file named `dmo.sql` like this:

```shell
curl -XPOST -d @dmo.sql https://api.kaveland.no/eugene/app/lint.raw
unnamed:1 E2 Validating table with a new `NOT NULL` column https://kaveland.no/eugene/hints/E2/
unnamed:1 E9 Taking dangerous lock without timeout https://kaveland.no/eugene/hints/E9/
unnamed:2 E2 Validating table with a new `NOT NULL` column https://kaveland.no/eugene/hints/E2/
unnamed:2 E4 Running more statements after taking `AccessExclusiveLock` https://kaveland.no/eugene/hints/E4/
unnamed:2 E9 Taking dangerous lock without timeout https://kaveland.no/eugene/hints/E9/
unnamed:2 W12 Multiple `ALTER TABLE` statements where one will do https://kaveland.no/eugene/hints/W12/
```

### `POST /lint.json`

This endpoint accepts a json body:

```json
{
  "sql": "-- name.sql\ncreate table books (id serial primary key);"
}
```

The `sql` member is a single SQL script. It responds with a json object that
contains the results, suitable to use for rendering templates or something.

```
{
  "name": "dmo.sql",
  "passed_all_checks": false,
  "skip_summary": false  
  "statements": [
    {
      "statement_number": 1,
      "line_number": 1,
      "sql": "alter table books\n  alter column text set not null",
      "triggered_rules": [
        {
          "id": "E2",
          "name": "Validating table with a new `NOT NULL` column",
          "condition": "A column was changed from `NULL` to `NOT NULL`",
...          
```

### Usage
If you find yourself using the API a lot, please consider installing `eugene` locally instead.
