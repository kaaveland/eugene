---
title: E2 Validating table with a new `NOT NULL` column
weight: 2
---

# E2 Validating table with a new `NOT NULL` column

## Triggered when

A column was changed from `NULL` to `NOT NULL`.

## Effect

This blocks all table access until all rows are validated.

## Workaround

Add a `CHECK` constraint as `NOT VALID`, validate it later, then make the column `NOT NULL`.

## Support

This hint is supported by `eugene lint`, `eugene trace`.

