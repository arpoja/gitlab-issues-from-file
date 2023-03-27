# Gitlab issues from file
- Learning rust with this simple project
- Creates issues in gitlab from csv or json files
# Usage
- `gitlab-issues-from-file --help`
- Basic example. Set `GITLAB_URL`, `GITLAB_ACCESS_TOKEN` environment variables and run `gitlab-issues-from-file -f issues.csv -p myproject`
- Logging is done using [env_logger](https://docs.rs/env_logger/latest/env_logger/). Set `RUST_LOG` environment variable to `debug` to see debug logs. `--verbose` sets level to `info` and normal logs are `warn` and `error`
# Current status
- [x] Let user choose the project to create the issues in
    - [x] Let user choose the project by id
    - [x] Let user choose the project by name
        - If there are multiple projects with the same name, we error out and ask the user to use the full path
    - [x] Let user choose the project by full path (namespace/project)

- [x] Create issues from csv file
    - [x] Let user choose the column to use as title, either by name or index
    - [x] Let user choose the column to use as description, either by name or index
- [x] Create issues from json file
    - [x] Let user choose the key to use as title
    - [x] Let user choose the key to use as description
- [x] Let user choose labels to add to the issues
- [x] Let user choose assignee to add to the issues
- [ ] Let user choose milestone to add to the issues
