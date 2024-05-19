---
title: E7 Creating a new unique constraint
weight: 7
---

# E7 Creating a new unique constraint

## Triggered when

Found a new unique constraint and a new index.

## Effect

This blocks all writes to the table while the index is being created and validated.

## Workaround

`CREATE UNIQUE INDEX CONCURRENTLY`, then add the constraint using the index.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

