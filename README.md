# Gitlab issues from file
- Learning rust with this simple project
- Creates issues in gitlab from csv or json files
# Usage
- `gitlab-issues-from-file --help`
- Basic example. Set `GITLAB_URL`, `GITLAB_ACCESS_TOKEN` environment variables and run `gitlab-issues-from-file -f issues.csv -p myproject`
- Logging is done using [env_logger](https://docs.rs/env_logger/latest/env_logger/). Set `RUST_LOG` environment variable to `debug` to see debug logs. `--verbose` sets level to `info` and normal logs are `warn` and `error`
# Current status
- [x] Let user choose the project to create the issues in (by id, name or path)
- Parsing options:
    - [x] parse csv file
    - [x] parse json file
    - [x] choose the separator for csv files
    - [x] choose the key (or index for csv) to use as title
    - [x] choose the key (or index for csv) to use as description
    - [x] choose to combine all non-title keys into a single description
- [x] Let user choose labels to add to the issues
- [x] Let user choose assignee to add to the issues
- [ ] Let user choose milestone to add to the issues
