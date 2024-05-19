# Creating a new exclusion constraint

## Triggered when

Found a new exclusion constraint.

## Effect

This blocks all reads and writes to the table while the constraint index is being created.

## Workaround

There is no safe way to add an exclusion constraint to an existing table.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

