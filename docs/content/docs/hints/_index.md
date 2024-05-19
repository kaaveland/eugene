+++
title = "eugene hints"
weight = 100
+++

# eugene hints

This section contains a list of hints that eugene recognizes, whether they are supported by `eugene lint`, 
`eugene trace`, or both, and what you can do to avoid the dangerous pattern.

## Structure

Each hint chapter starts with a general description of when the hint is triggered,
what effect the pattern has on the database and concurrent transactions, then
if a workaround exists, it will be described. The hint chapter will also tell you
whether the hint is detected by `eugene lint`, `eugene trace`, or both.

Subchapters show an example trace report and example lint report for each hint, 
to help you understand what the detected pattern looks like in SQL code. 
For hints where eugene knows about a workaround, there is also a subchapter
showing a trace report and a lint report for the safer way to achieve the same
result.

## Report structure

Each pattern is show in a way so that all the SQL could be run in an empty
database, and the pattern would still be detected. All hints are shown by
using several SQL scripts, number from `1.sql` and up and they are run
in ascending order. Each report then has a heading for every file, and
for every file, there is a heading for every statement in the file.
Navigation on the right side of the page will help you navigate the report.
