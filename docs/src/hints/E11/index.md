# Adding a `SERIAL` or `GENERATED ... STORED` column

## Triggered when

A new column was added with a `SERIAL` or `GENERATED` type.

## Effect

This blocks all table access until the table is rewritten.

## Workaround

Can not be done without a table rewrite.

## Support

This hint is supported by `eugene lint`.

