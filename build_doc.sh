#!/usr/bin/env bash

set -e -u -f -o pipefail

sed -i '' -e 's/[[:space:]]*$//' docs/src/SUMMARY.md
cat docs/src/generated_hint_toc.md >> docs/src/SUMMARY.md
mdbook build docs && git checkout -- docs/src/SUMMARY.md
