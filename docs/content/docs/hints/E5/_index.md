---
title: E5 Type change requiring table rewrite
weight: 5
---

# E5 Type change requiring table rewrite

## Triggered when

A column was changed to a data type that isn't binary compatible.

## Effect

This causes a full table rewrite while holding a lock that prevents all other use of the table.

## Workaround

Add a new column, update it in batches, and drop the old column.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

