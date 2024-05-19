# Creating an enum

## Triggered when

A new enum was created.

## Effect

Removing values from an enum requires difficult migrations, and associating more data with an enum value is difficult.

## Workaround

Use a foreign key to a lookup table instead.

## Support

This hint is supported by `eugene lint`.

