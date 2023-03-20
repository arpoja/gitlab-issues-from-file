use clap::Parser;
use csv;
use json;
use serde_json;
use std::{collections::HashMap, rc::Rc};
use uuid::{uuid, Uuid};

const SUPPORTED_FILE_TYPES: [&'static str; 2] = ["csv", "json"];
const DEFAULT_GITLAB_URL: &'static str = "https://localhost";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the file to upload. Required.
    #[arg(short, long, value_name = "FILE", required = true)]
    file: Option<std::path::PathBuf>,

    /// Field separator to use when parsing a csv file.
    /// Defaults to comma.
    /// Ignored if file is not a csv file.
    #[arg(short, long, default_value = ",")]
    separator: Option<char>,

    /// URL of the GitLab instance, e.g. https://gitlab.com.
    /// Defaults to https://localhost.
    #[arg(short, long, default_value = DEFAULT_GITLAB_URL)]
    url: Option<String>,

    /// GitLab API token. If not provided,
    /// the GITLAB_ACCESS_TOKEN environment variable is used.
    /// If neither is provided, you will be prompted for one.
    #[arg(short, long)]
    token: Option<String>,

    /// Name of the gitlab project to upload to.
    /// Required if project_id is not provided.
    #[arg(short, long)]
    project_name: Option<String>,

    /// ID of the gitlab project to upload to.
    /// Required if project_name is not provided.
    #[arg(long)]
    project_id: Option<u64>,

    /// Comma separated list of labels to add to the issue.
    /// None are added by default.
    #[arg(short, long)]
    labels: Option<String>,

    /// Assignee username to add to the issue.
    /// None are added by default.
    #[arg(short, long)]
    assignee: Option<String>,

    /// Should we disable SSL verification for requests to gitlab?
    /// Defaults to false.
    #[arg(short, long, default_value = "false")]
    no_ssl_verify: Option<bool>,

    /// Check if the file can be used to extract gitlab tasks.
    /// No upload is performed. Defaults to false.
    #[arg(short, long, default_value = "false")]
    check: Option<bool>,

    /// Verbose output. Defaults to false.
    #[arg(short, long, default_value = "false")]
    verbose: Option<bool>,
}

fn verify_args(args: &mut Args) {
    // Verify that the file exists and is a file
    if args.file.is_none() {
        eprintln!("File must be provided");
        std::process::exit(1);
    } else if !args.file.as_ref().unwrap().exists() {
        eprintln!("File does not exist");
        std::process::exit(1);
    } else if !args.file.as_ref().unwrap().is_file() {
        eprintln!("File is not a file");
        std::process::exit(1);
    } else {
        // Check if the file type is supported
        let file_type = args.file.as_ref().unwrap().extension().unwrap();
        if !SUPPORTED_FILE_TYPES.contains(&file_type.to_ascii_lowercase().to_str().unwrap()) {
            eprintln!("File type is not supported");
            std::process::exit(1);
        }
    }
    // Verify that either url is provided or GITLAB_URL is set
    if args.url == Some(DEFAULT_GITLAB_URL.to_string()) {
        if let Ok(url) = std::env::var("GITLAB_URL") {
            args.url = Some(url);
        } else {
            eprintln!("Either url by argument or GITLAB_URL environmentString environment variable must be provided");
            std::process::exit(1);
        }
    }
    // Verify that either project_name or project_id is provided
    if args.project_name.is_none() && args.project_id.is_none() {
        eprintln!("Either project_name or project_id must be provided");
        std::process::exit(1);
    }
    if args.project_name.is_some() && args.project_id.is_some() {
        eprintln!("Only one of project_name or project_id can be provided");
        std::process::exit(1);
    }
    // Verify that labels is a comma separated list
    if args.labels.is_some() {
        let labels = args.labels.as_ref().unwrap();
        if labels.contains(",") {
            let labels: Vec<&str> = labels.split(",").collect();
            for label in labels {
                if label.is_empty() {
                    eprintln!("Labels must be a comma separated list of non-empty strings");
                    std::process::exit(1);
                }
            }
        }
    }
}

struct GitLabProjectImportantInfo {
    id: u64,
    name: String,
    path_with_namespace: String,
}
struct GitLabMemberImportantInfo {
    id: u64,
    username: String,
    name: String,
}
struct GitLabLabelImportantInfo {
    id: u64,
    name: String,
}
struct GitLabApiRequest {
    url: String,
    token: String,
    no_ssl_verify: bool,
}
impl GitLabApiRequest {
    fn get_projects(&self) -> Result<Vec<GitLabProjectImportantInfo>, &'static str> {
        // We know that our struct is valid, so we dont need to check for errors
        let url = format!("{}/projects", self.url);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("PRIVATE-TOKEN", self.token.parse().unwrap());
        let client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(self.no_ssl_verify)
            .build()
            .unwrap();
        let response = match client.get(&url).headers(headers).send() {
            Ok(response) => response,
            Err(_) => return Err("Failed to send request"),
        };
        // Check if the response was successful
        if !response.status().is_success() {
            return Err("Request was not successful");
        }
        // Parse the response with serde before turning the important info into a vector of structs
        let projects_array: Vec<serde_json::Value> = match response.json() {
            Ok(projects_array) => projects_array,
            Err(_) => return Err("Failed to parse response"),
        };
        let mut projects: Vec<GitLabProjectImportantInfo> = Vec::new();
        for project in projects_array {
            let p = GitLabProjectImportantInfo {
                id: project["id"].as_u64().unwrap(),
                name: project["name"].as_str().unwrap().to_string(),
                path_with_namespace: project["path_with_namespace"].as_str().unwrap().to_string(),
            };
            projects.push(p);
        }

        Ok(projects)
    }

    fn get_members_of_project(
        &self,
        project_id: u64,
    ) -> Result<Vec<GitLabMemberImportantInfo>, &'static str> {
        let url = format!("{}/projects/{}/members", self.url, project_id);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("PRIVATE-TOKEN", self.token.parse().unwrap());
        let client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(self.no_ssl_verify)
            .build()
            .unwrap();
        let response = match client.get(&url).headers(headers).send() {
            Ok(response) => response,
            Err(_) => return Err("Failed to send request"),
        };
        // Check if the response was successful
        if !response.status().is_success() {
            return Err("Request was not successful");
        }
        // Parse the response with serde before turning the important info into a vector of structs
        let members_array: Vec<serde_json::Value> = match response.json() {
            Ok(members) => members,
            Err(_) => return Err("Failed to parse response"),
        };
        let mut members: Vec<GitLabMemberImportantInfo> = Vec::new();
        for member in members_array {
            let m = GitLabMemberImportantInfo {
                id: member["id"].as_u64().unwrap(),
                username: member["username"].as_str().unwrap().to_string(),
                name: member["name"].as_str().unwrap().to_string(),
            };
            members.push(m);
        }

        Ok(members)
    }
    fn get_labels_of_project(
        &self,
        project_id: u64,
    ) -> Result<Vec<GitLabLabelImportantInfo>, &'static str> {
        let url = format!("{}/projects/{}/labels", self.url, project_id);
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("PRIVATE-TOKEN", self.token.parse().unwrap());
        let client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(self.no_ssl_verify)
            .build()
            .unwrap();
        let response = match client.get(&url).headers(headers).send() {
            Ok(response) => response,
            Err(_) => return Err("Failed to send request"),
        };
        // Check if the response was successful
        if !response.status().is_success() {
            return Err("Request was not successful");
        }

        let labels_array: Vec<serde_json::Value> = match response.json() {
            Ok(labels) => labels,
            Err(_) => return Err("Failed to parse response"),
        };

        let mut labels = Vec::new();
        for label in labels_array {
            let l = GitLabLabelImportantInfo {
                id: label["id"].as_u64().unwrap(),
                name: label["name"].as_str().unwrap().to_string(),
            };
            labels.push(l);
        }
        Ok(labels)
    }
    fn post_issue(&self, issue: &GitLabIssue) -> Result<(), &'static str> {
        todo!("Post an issue to the GitLab API")
    }
}

struct GitLabIssue {
    id: Uuid,
    project_id: u64,
    title: String,
    description: String,
    labels: Option<String>,
    assignee: Option<String>,
}
impl GitLabIssue {
    fn new(project_id: u64, issue: Issue) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            title: issue.title,
            description: issue.description,
            labels: None,
            assignee: None,
        }
    }
    fn set_labels(&mut self, labels: String) {
        self.labels = Some(labels);
    }
    fn set_assignee(&mut self, assignee: String) {
        self.assignee = Some(assignee);
    }
}

struct Issue {
    title: String,
    description: String,
}

fn find_matching_header(headers: &csv::StringRecord, our_header: &str) -> Option<usize> {
    todo!("Find the index of the header in the csv file")
}

fn find_matching_attribute(attributes: &json::JsonValue, out_attribute: &str) -> Option<usize> {
    todo!("Find the index of the attribute in the json file")
}

fn ask_user_for_token(args: &Args) -> Result<String, &'static str> {
    let mut buffer = String::new();
    if args.verbose.unwrap() {
        println!("No token provided. Please enter your GitLab API token:");
    }
    let token = match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => buffer.trim().to_string(),
        Err(_) => return Err("Could not read token"),
    };
    Ok(token)
}

fn ask_user_for_header(
    headers: &csv::StringRecord,
    wanted_header_name: &str,
) -> Result<usize, &'static str> {
    let mut buffer = String::new();
    println!("No '{}' header found in the csv file:", wanted_header_name);
    for (i, header) in headers.iter().enumerate() {
        println!("{}: {}", i, header);
    }
    println!(
        "Please enter the number of the header you wish to use for issue {}:",
        wanted_header_name
    );

    let index = match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => buffer.trim().parse::<usize>().unwrap(),
        Err(_) => return Err("Could not parse input"),
    };
    if index >= headers.len() {
        return Err("Index out of bounds");
    }
    Ok(index)
}

fn file_to_issues(args: &Args) -> Option<Vec<Issue>> {
    let file_type = args.file.as_ref().unwrap().extension().unwrap();
    let file = args.file.as_ref().unwrap();
    let separator = args.separator.as_ref().unwrap();
    match file_type.to_ascii_lowercase().to_str().unwrap() {
        "csv" => csv_to_issues(file, separator),
        "json" => json_to_issues(file),
        _ => None,
    }
}

fn csv_to_issues(file: &std::path::PathBuf, separator: &char) -> Option<Vec<Issue>> {
    let mut issues: Vec<Issue> = Vec::new();
    let mut reader = csv::Reader::from_path(file).unwrap();
    let headers = match reader.headers() {
        Ok(headers) => headers,
        Err(e) => {
            eprintln!("Could not read headers from csv file: {}", e);
            std::process::exit(1);
        }
    };
    // Check if title header exists. If not, ask the user which header should be used for title
    let title_index = match find_matching_header(&headers, "title") {
        Some(index) => index,
        None => loop {
            match ask_user_for_header(&headers, "title") {
                Ok(index) => break index,
                Err(e) => {
                    eprintln!("Could not parse input: {}", e);
                    continue;
                }
            }
        },
    };
    // Check if description header exists. If not, ask the user which header should be used for description
    let description_index = match find_matching_header(&headers, "description") {
        Some(index) => index,
        None => loop {
            match ask_user_for_header(&headers, "description") {
                Ok(index) => break index,
                Err(e) => {
                    eprintln!("Could not parse input: {}", e);
                    continue;
                }
            }
        },
    };
    for result in reader.records() {
        let record = match result {
            Ok(record) => record,
            Err(e) => {
                eprintln!("Could not read record from csv file: {}", e);
                continue;
            }
        };
        let title = match record.get(title_index) {
            Some(title) => title,
            None => {
                eprintln!("Could not read title from csv file");
                std::process::exit(1);
            }
        };
        let description = match record.get(description_index) {
            Some(description) => description,
            None => {
                eprintln!("Could not read description from csv file");
                std::process::exit(1);
            }
        };
        issues.push(Issue {
            title: title.to_string(),
            description: description.to_string(),
        });
    }
    Some(issues)
}

fn json_to_issues(file: &std::path::PathBuf) -> Option<Vec<Issue>> {
    todo!("Implement json_to_issues");
}

fn project_id_exists(project_id: &u64, projects: &Vec<GitLabProjectImportantInfo>) -> bool {
    for project in projects {
        if project.id == *project_id {
            return true;
        }
    }
    false
}

fn get_project_id_from_name(
    project_name: &str,
    projects: &Vec<GitLabProjectImportantInfo>,
) -> Option<u64> {
    let mut res: Vec<u64> = Vec::new();
    for project in projects {
        if project.name == project_name {
            res.push(project.id);
        }
    }
    if res.len() == 1 {
        Some(res[0])
    } else if res.len() > 1 {
        eprintln!(
            "Multiple projects with the same name {} found",
            project_name
        );
        None
    } else if res.len() == 0 {
        eprintln!("No project with the name '{}' found", project_name);
        None
    } else {
        eprintln!("Something went wrong");
        None
    }
}

fn project_member_exists(username: &str, project_members: &Vec<GitLabMemberImportantInfo>) -> bool {
    for project_member in project_members {
        if project_member.username == username {
            return true;
        }
    }
    false
}

fn project_label_exists(label: &str, project_labels: &Vec<GitLabLabelImportantInfo>) -> bool {
    for project_label in project_labels {
        if project_label.name == label {
            return true;
        }
    }
    false
}

fn main() {
    let mut args = Args::parse();
    verify_args(&mut args);

    // Verify that the file can be used to extract gitlab tasks
    // Create a list of issues from the file
    let mut issues: Vec<Issue> = Vec::new();
    if let Some(issues_from_file) = file_to_issues(&args) {
        issues.extend(issues_from_file); // TODO: Check if this is correct
    } else {
        eprintln!("File could not be used to extract gitlab tasks");
        std::process::exit(1);
    }
    if args.verbose.unwrap() {
        println!(
            "Found {} issues in the file {}",
            issues.len(),
            args.file.as_ref().unwrap().to_str().unwrap()
        );
        for issue in &issues {
            println!("\tTitle: {}", issue.title);
            println!("\tDescription: {}", issue.description);
        }
    }
    // Exit if check is true, i.e. we don't want to upload
    if args.check.unwrap() {
        println!("Exiting before upload because you requested a check only.");
        std::process::exit(0);
    }
    // Check if the user provided a token. If not, ask the user for one
    let token = match args.token.as_ref() {
        Some(token) => token.to_string(),
        None => loop {
            match ask_user_for_token(&args) {
                Ok(token) => break token,
                Err(e) => {
                    eprintln!("Could not read token: {}", e);
                    continue;
                }
            }
        },
    };
    // Build the gitlab request struct
    let api_url = format!("{}/api/v4", args.url.as_ref().unwrap()); // we know that url is Some() because we verified the args
    let gitlab_request = GitLabApiRequest {
        url: api_url,
        token,
        no_ssl_verify: args.no_ssl_verify.unwrap(), // we know that no_ssl_verify is Some() because it has a default value
    };
    // Verify that the gitlab url and token are valid by getting the available projects from gitlab api
    let projects = match gitlab_request.get_projects() {
        Ok(projects) => projects,
        Err(e) => {
            eprintln!("Could not get projects from gitlab: {}", e);
            std::process::exit(1);
        }
    };
    if args.verbose.unwrap() {
        println!("Found {} projects in gitlab:", projects.len());
        for project in &projects {
            println!(
                "\tid: {}, Name: {}, Name with path: {}",
                project.id, project.name, project.path_with_namespace
            );
        }
    }
    // Check if the user provided project name or id exists in response from gitlab api
    if args.project_id.is_none() {
        // We need to get the project id from the project name
        match get_project_id_from_name(args.project_name.as_ref().unwrap(), &projects) {
            Some(project_id) => args.project_id = Some(project_id),
            None => {
                eprintln!(
                    "Could not find project with name {}",
                    args.project_name.as_ref().unwrap()
                );
                std::process::exit(1);
            }
        };
    } else if !project_id_exists(&args.project_id.unwrap(), &projects) {
        eprintln!(
            "Could not find project with id {}",
            args.project_id.unwrap()
        );
        std::process::exit(1);
    }
    // We have a valid project id
    let project_id = args.project_id.unwrap();
    // Check if the user provided an assignee and if it exists in the project
    if args.assignee.is_some() {
        let assignee = args.assignee.as_ref().unwrap().to_string(); // we know that assignee is Some() because we verified the args
        let project_members = match gitlab_request.get_members_of_project(project_id) {
            Ok(members) => members,
            Err(e) => {
                eprintln!("Could not get members from gitlab: {}", e);
                std::process::exit(1);
            }
        };
        if args.verbose.unwrap() {
            println!(
                "Found {} members in project with id {}",
                project_members.len(),
                project_id
            );
            for member in &project_members {
                println!("\tid: {}, username: {}", member.id, member.username);
            }
        }
        if !project_member_exists(&assignee, &project_members) {
            eprintln!(
                "Could not find assignee {} in project with id {}",
                assignee, project_id
            );
            std::process::exit(1);
        }
        if args.verbose.unwrap() {
            println!(
                "Assignee {} exists in project with id {}",
                assignee, project_id
            );
        }
    }

    // Check if the user provided labels and if they exist in the project
    if args.labels.is_some() {
        // Verify that the labels are a comma separated list
        let labels = args.labels.as_ref().unwrap().to_string(); // we know that labels is Some() because we verified the args
        let labels: Vec<&str> = labels.split(',').map(|s| s.trim()).collect();

        // Get the labels from the project
        let project_labels = match gitlab_request.get_labels_of_project(project_id) {
            Ok(labels) => labels,
            Err(e) => {
                eprintln!("Could not get labels from gitlab: {}", e);
                std::process::exit(1);
            }
        };
        if args.verbose.unwrap() {
            println!(
                "Found {} labels in project with id {}",
                project_labels.len(),
                project_id
            );
            for label in &project_labels {
                println!("\tid: {}, name: {}", label.id, label.name);
            }
        }
        // Check that each label exists in the project
        let mut count = 0;
        for label in &labels {
            for pl in &project_labels {
                if pl.name == *label {
                    count += 1;
                    break;
                }
            }
        }
        if count != labels.len() {
            eprintln!(
                "Could not find all labels in project with id {}",
                project_id
            );
            std::process::exit(1);
        }
        if args.verbose.unwrap() {
            println!("All labels exist in project with id {}", project_id);
        }
    }
    // We are ready to upload the issues to gitlab
    for issue in issues {
        let mut gitlab_issue = GitLabIssue::new(args.project_id.unwrap(), issue); // we know that project_id is Some() because we verified the args
        if args.assignee.is_some() {
            let assignee = args.assignee.as_ref().unwrap().to_string(); // we know that assignee is Some() because we verified the args
            gitlab_issue.set_assignee(assignee);
        }
        if args.labels.is_some() {
            let labels = args.labels.as_ref().unwrap().to_string(); // we know that labels is Some() because we verified the args
            gitlab_issue.set_labels(labels);
        }
        match gitlab_request.post_issue(&gitlab_issue) {
            Ok(_) => println!("TODO"),
            Err(e) => eprintln!("Could not create issue: {}", e),
        }
    }
}
