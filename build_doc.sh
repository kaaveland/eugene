#!/usr/bin/env bash

set -e -u -f -o pipefail

sed -i '' -e '${/^$/d;}' docs/src/SUMMARY.md || true
cat docs/src/generated_hint_toc.md >> docs/src/SUMMARY.md

if [[ $# -eq 0 ]]; then
  if [[ -x "./mdbook" ]]; then
    ./mdbook build docs
  else
    mdbook build docs
  fi
  git checkout -- docs/src/SUMMARY.md
else
  if [[ -x "./mdbook" ]]; then
    ./mdbook serve docs
  else
    mdbook serve docs
  fi
fi
