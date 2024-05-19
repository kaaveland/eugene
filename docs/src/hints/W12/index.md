# Multiple `ALTER TABLE` statements where one will do

## Triggered when

Multiple `ALTER TABLE` statements targets the same table.

## Effect

If the statements require table scans, there will be more scans than necessary.

## Workaround

Combine the statements into one, separating the action with commas.

## Support

This hint is supported by `eugene lint`.

