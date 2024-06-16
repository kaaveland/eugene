# Running in GitHub Actions

With the `--git-diff` option, it is easy to set up eugene to run in a GitHub Actions workflow.
Below are some example jobs that you copy to your github workflows. There are 4 different jobs
configured:

- `trace` will run `eugene trace` on the files that have changed since `main` and stop the build
  if it finds any issues.
- `lint` will run `eugene lint` on the files that have changed since `main` and stop the build
  if it finds any issues.
- `post_trace` will run `eugene trace` on the files that have changed since `main` and post the
  results as a markdown comment on the pull request, but allow the build to pass even if issues
  are found.
- `post_lint` will run `eugene lint` on the files that have changed since `main` and post the
  results as a markdown comment on the pull request, but allow the build to pass even if issues
  are found.

```yaml
name: Eugene CI check
on:
  pull_request:
    branches:
      - main
env:
  EUGENE_VERSION: "0.6.1"

permissions:
  contents: read
  pull-requests: write

jobs:
  trace:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
      with:
        fetch-depth: 0
    - name: Download eugene
      run: |
        curl -L  https://github.com/kaaveland/eugene/releases/download/$EUGENE_VERSION/eugene-x86_64-unknown-linux-musl -o eugene
        chmod +x eugene
    - name: Put postgres binaries on PATH for eugene
      run: echo "/usr/lib/postgresql/14/bin" >> $GITHUB_PATH
    - name: Trace
      run: ./eugene trace --git-diff origin/main migration-scripts

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Download eugene
        run: |
          curl -L  https://github.com/kaaveland/eugene/releases/download/$EUGENE_VERSION/eugene-x86_64-unknown-linux-musl -o eugene
          chmod +x eugene
      - name: Lint files
        run: ./eugene lint --git-diff origin/main migration-scripts

  post_trace:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Download eugene
        run: |
          curl -L  https://github.com/kaaveland/eugene/releases/download/$EUGENE_VERSION/eugene-x86_64-unknown-linux-musl -o eugene
          chmod +x eugene
      - name: Put postgres binaries on PATH for eugene
        run: echo "/usr/lib/postgresql/14/bin" >> $GITHUB_PATH
      - name: Trace files
        run: ./eugene trace --git-diff origin/main migration-scripts -f md --accept-failures > trace.md
      - name: Post Comment
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          COMMENT=$(cat trace.md)
          gh pr comment ${{ github.event.pull_request.number }} --body "$COMMENT"

  post_lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: actions/checkout@v4
      - name: Download eugene
        run: |
          curl -L  https://github.com/kaaveland/eugene/releases/download/$EUGENE_VERSION/eugene-x86_64-unknown-linux-musl -o eugene
          chmod +x eugene
      - name: Lint files
        run: ./eugene lint --git-diff origin/main migration-scripts -f md --accept-failures > lint.md
      - name: Post Comment
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          COMMENT=$(cat lint.md)
          gh pr comment ${{ github.event.pull_request.number }} --body "$COMMENT"
```
