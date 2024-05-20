# eugene rules

This section contains a list of hints that eugene recognizes, whether they
are supported by `eugene lint`, `eugene trace`, or both, and what you can
do to avoid the dangerous pattern.

These are all automatically generated from templates during the build of
`eugene` so when new hints are added, the documentation will be updated
automatically.

Each chapter refers to a specific **rule** in the `eugene` codebase. The
rule is identified by an ID that can be passed to `eugene`, as well as a
name. The documentation describes what `eugene` looks for when triggering
the rule, what effect the schema change may have on the database and
concurrent transactions, and if there is a workaround. The documentation
will tell you whether one or both of `eugene lint` and `eugene trace`
that can detect the condition the rule describes.

Each rule page will link to some eugene example reports for the migration
pattern it describes, so you can see what the output looks like.
