---
title: E10 Rewrote table or index while holding dangerous lock
weight: 10
---

# E10 Rewrote table or index while holding dangerous lock

## Triggered when

A table or index was rewritten while holding a lock that blocks many operations.

## Effect

This blocks many operations on the table or index while the rewrite is in progress.

## Workaround

Build a new table or index, write to both, then swap them.

## Support

This hint is supported by `eugene trace`.

