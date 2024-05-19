# Adding a primary key using an index

## Triggered when

A primary key was added using an index on the table.

## Effect

This can cause postgres to alter the index columns to be `NOT NULL`.

## Workaround

Make sure that all the columns in the index are already `NOT NULL`.

## Support

This hint is supported by `eugene lint`.

