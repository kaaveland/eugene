# Add a new JSON column

## Triggered when

A new column of type `json` was added to a table.

## Effect

This breaks `SELECT DISTINCT` queries or other operations that need equality checks on the column.

## Workaround

Use the `jsonb` type instead, it supports all use-cases of `json` and is more robust and compact.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

