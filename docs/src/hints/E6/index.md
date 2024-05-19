# Creating a new index on an existing table

## Triggered when

A new index was created on an existing table without the `CONCURRENTLY` keyword.

## Effect

This blocks all writes to the table while the index is being created.

## Workaround

Run `CREATE INDEX CONCURRENTLY` instead of `CREATE INDEX`.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

