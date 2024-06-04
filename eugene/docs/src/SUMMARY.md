# Summary

- [Introduction](introduction.md)
- [eugene lint](lint.md)
- [eugene trace](trace.md)
- [Ignoring hints](ignores.md)
- [Hint reference](hints.md)
  - [E1 Validating table with a new constraint](./hints/E1/index.md)
  - [E2 Validating table with a new `NOT NULL` column](./hints/E2/index.md)
  - [E3 Add a new JSON column](./hints/E3/index.md)
  - [E4 Running more statements after taking `AccessExclusiveLock`](./hints/E4/index.md)
  - [E5 Type change requiring table rewrite](./hints/E5/index.md)
  - [E6 Creating a new index on an existing table](./hints/E6/index.md)
  - [E7 Creating a new unique constraint](./hints/E7/index.md)
  - [E8 Creating a new exclusion constraint](./hints/E8/index.md)
  - [E9 Taking dangerous lock without timeout](./hints/E9/index.md)
  - [E10 Rewrote table or index while holding dangerous lock](./hints/E10/index.md)
  - [E11 Adding a `SERIAL` or `GENERATED ... STORED` column](./hints/E11/index.md)
  - [W12 Multiple `ALTER TABLE` statements where one will do](./hints/W12/index.md)
  - [W13 Creating an enum](./hints/W13/index.md)
  - [W14 Adding a primary key using an index](./hints/W14/index.md)
---------
- [Example Reports](./hints/examples.md)
  - [E1 lint problematic](./hints/E1/unsafe_lint.md)
  - [E1 lint safer](./hints/E1/safer_lint.md)
  - [E1 trace problematic](./hints/E1/unsafe_trace.md)
  - [E1 trace safer](./hints/E1/safer_trace.md)
  - [E2 lint problematic](./hints/E2/unsafe_lint.md)
  - [E2 lint safer](./hints/E2/safer_lint.md)
  - [E2 trace problematic](./hints/E2/unsafe_trace.md)
  - [E2 trace safer](./hints/E2/safer_trace.md)
  - [E3 lint problematic](./hints/E3/unsafe_lint.md)
  - [E3 lint safer](./hints/E3/safer_lint.md)
  - [E3 trace problematic](./hints/E3/unsafe_trace.md)
  - [E3 trace safer](./hints/E3/safer_trace.md)
  - [E4 lint problematic](./hints/E4/unsafe_lint.md)
  - [E4 lint safer](./hints/E4/safer_lint.md)
  - [E4 trace problematic](./hints/E4/unsafe_trace.md)
  - [E4 trace safer](./hints/E4/safer_trace.md)
  - [E5 lint problematic](./hints/E5/unsafe_lint.md)
  - [E5 lint safer](./hints/E5/safer_lint.md)
  - [E5 trace problematic](./hints/E5/unsafe_trace.md)
  - [E5 trace safer](./hints/E5/safer_trace.md)
  - [E6 lint problematic](./hints/E6/unsafe_lint.md)
  - [E6 lint safer](./hints/E6/safer_lint.md)
  - [E6 trace problematic](./hints/E6/unsafe_trace.md)
  - [E6 trace safer](./hints/E6/safer_trace.md)
  - [E7 lint problematic](./hints/E7/unsafe_lint.md)
  - [E7 lint safer](./hints/E7/safer_lint.md)
  - [E7 trace problematic](./hints/E7/unsafe_trace.md)
  - [E7 trace safer](./hints/E7/safer_trace.md)
  - [E8 lint problematic](./hints/E8/unsafe_lint.md)
  - [E8 trace problematic](./hints/E8/unsafe_trace.md)
  - [E9 lint problematic](./hints/E9/unsafe_lint.md)
  - [E9 lint safer](./hints/E9/safer_lint.md)
  - [E9 trace problematic](./hints/E9/unsafe_trace.md)
  - [E9 trace safer](./hints/E9/safer_trace.md)
  - [E10 lint problematic](./hints/E10/unsafe_lint.md)
  - [E10 lint safer](./hints/E10/safer_lint.md)
  - [E10 trace problematic](./hints/E10/unsafe_trace.md)
  - [E10 trace safer](./hints/E10/safer_trace.md)
  - [E11 lint problematic](./hints/E11/unsafe_lint.md)
  - [E11 trace problematic](./hints/E11/unsafe_trace.md)
  - [W12 lint problematic](./hints/W12/unsafe_lint.md)
  - [W12 lint safer](./hints/W12/safer_lint.md)
  - [W12 trace problematic](./hints/W12/unsafe_trace.md)
  - [W12 trace safer](./hints/W12/safer_trace.md)
  - [W13 lint problematic](./hints/W13/unsafe_lint.md)
  - [W13 lint safer](./hints/W13/safer_lint.md)
  - [W13 trace problematic](./hints/W13/unsafe_trace.md)
  - [W13 trace safer](./hints/W13/safer_trace.md)
  - [W14 lint problematic](./hints/W14/unsafe_lint.md)
  - [W14 lint safer](./hints/W14/safer_lint.md)
  - [W14 trace problematic](./hints/W14/unsafe_trace.md)
  - [W14 trace safer](./hints/W14/safer_trace.md)