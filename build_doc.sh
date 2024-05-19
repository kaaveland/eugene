#!/usr/bin/env bash

set -e -u -f -o pipefail

sed -i '' -e '${/^$/d;}' docs/src/SUMMARY.md || true
cat docs/src/generated_hint_toc.md >> docs/src/SUMMARY.md
mdbook build docs && git checkout -- docs/src/SUMMARY.md
