# Validating table with a new constraint

## Triggered when

A new constraint was added and it is already `VALID`.

## Effect

This blocks all table access until all rows are validated.

## Workaround

Add the constraint as `NOT VALID` and validate it with `ALTER TABLE ... VALIDATE CONSTRAINT` later.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

