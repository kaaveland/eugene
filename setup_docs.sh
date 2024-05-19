#!/usr/bin/env bash

set -e -u -f -o pipefail

mkdir -p docs/themes

echo "Adding book theme as a submodule"

git submodule add --force --depth=1 https://github.com/alex-shpak/hugo-book.git docs/themes/book || true
git submodule update --init --recursive
git submodule set-branch --branch v10 docs/themes/book
