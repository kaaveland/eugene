# Taking dangerous lock without timeout

## Triggered when

A lock that would block many common operations was taken without a timeout.

## Effect

This can block all other operations on the table indefinitely if any other transaction holds a conflicting lock while `idle in transaction` or `active`.

## Workaround

Run `SET LOCAL lock_timeout = '2s';` before the statement and retry the migration if necessary.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

