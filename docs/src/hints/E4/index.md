# Running more statements after taking `AccessExclusiveLock`

## Triggered when

A transaction that holds an `AccessExclusiveLock` started a new statement.

## Effect

This blocks all access to the table for the duration of this statement.

## Workaround

Run this statement in a new transaction.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

