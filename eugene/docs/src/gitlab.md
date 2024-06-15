# Running in GitLab CI

With the `--git-diff` option, it is easy to set up eugene to run in a GitLab CI/CD pipeline.
Below are some example jobs that you can copy into your `.gitlab-ci.yml` file. 

There are 6 different jobs configured:
- `lint` will run `eugene lint` on the files that have changed since `main` and stop the build if it finds any issues.
- `trace` will run `eugene trace` on the files that have changed since `main` and stop the build if it finds any issues.
- `trace_report` and `comment_trace` will run `eugene trace` on the files that have changed since `main` and post the 
  results as a markdown comment on the merge request, but allow the build to pass even if issues are found.
- `lint_report` and `comment_lint` will run `eugene lint` on the files that have changed since `main` and post the 
  results as a markdown comment on the merge request, but allow the build to pass even if issues are found.

Note that for `comment_trace` and `comment_lint` to work, `GITLAB_TOKEN` must be set in
CI/CD Variables in the GitLab project settings. It should be a token that has access
to the project, so that it can post comments on merge requests.

```yaml
.eugene_rules:
  rules:
    - if: $CI_PIPELINE_SOURCE == 'merge_request_event'

.eugene:
  extends: .eugene_rules
  before_script:
    - git config --global --add safe.directory $CI_PROJECT_DIR
    - git fetch --depth=1 origin main
  image:
    name: ghcr.io/kaaveland/eugene:latest
    entrypoint: ["/bin/sh", "-c"]

lint:
  extends: .eugene
  script: eugene lint --git-diff origin/main flywaystyle-sql

trace:
  extends: .eugene
  script: eugene trace --git-diff origin/main flywaystyle-sql

trace_report:
    extends: .eugene
    script: eugene trace --git-diff origin/main flywaystyle-sql -f md --accept-failures > trace.md
    artifacts:
      paths:
        - trace.md

comment_trace:
  extends: .eugene_rules
  image:
    name: registry.gitlab.com/gitlab-org/cli
    entrypoint: [ "/bin/sh", "-c" ]
  needs:
    - trace_report
  script:
    - body=$(cat trace.md)
    - glab mr note $CI_MERGE_REQUEST_IID --unique -m "$body"

lint_report:
  extends: .eugene
  script: eugene lint --git-diff origin/main flywaystyle-sql -f md --accept-failures > lint.md
  rules:
    - if: $CI_PIPELINE_SOURCE == 'merge_request_event'
  artifacts:
    paths:
      - lint.md

comment_lint:
  extends: .eugene_rules
  image:
    name: registry.gitlab.com/gitlab-org/cli
    entrypoint: ["/bin/sh", "-c"]
  needs:
    - lint_report
  script:
    - body=$(cat lint.md)
    - glab mr note $CI_MERGE_REQUEST_IID --unique -m "$body"
```
