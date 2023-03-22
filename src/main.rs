use clap::Parser;
// Local files
mod gitlabapi;
mod issuefile;

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
    /// Does the csv file have a header row?
    #[arg(long, default_value = "false")]
    no_header: bool,
    /// Column name to use as the title of the issue when parsing a csv or json file.
    #[arg(long, default_value = "title")]
    title_column: Option<String>,
    /// Column index *Starting from 0* to use as the title of the issue when parsing a csv file.
    /// Ignored if file is not a csv file.
    /// If both title_column and title_index are provided, title_index is used.
    #[arg(long)]
    title_index: Option<usize>,

    /// Column name to use as the description of the issue when parsing a csv or json file.
    #[arg(long, default_value = "description")]
    description_column: Option<String>,
    /// Column index *Starting from 0* to use as the description of the issue when parsing a csv file.
    /// Ignored if file is not a csv file.
    /// If both description_column and description_index are provided, description_index is used.
    #[arg(long)]
    description_index: Option<usize>,

    /// URL of the GitLab instance, e.g. https://gitlab.com.
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
    check: bool,

    /// Verbose output. Defaults to false.
    #[arg(short, long, default_value = "false")]
    verbose: bool,
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
        if !issuefile::SUPPORTED_FILE_TYPES
            .contains(&file_type.to_ascii_lowercase().to_str().unwrap())
        {
            eprintln!("File type is not supported");
            std::process::exit(1);
        }
        // Set separator to None if file is not a csv file
        if file_type != "csv" {
            args.separator = None;
        }
    }
    // Verify that either url is provided or GITLAB_URL is set
    if args.url == Some(DEFAULT_GITLAB_URL.to_string()) {
        if let Ok(url) = std::env::var("GITLAB_URL") {
            args.url = Some(url);
        } else {
            eprintln!("Missing gitlab url. Either url by argument -u <URL> or GITLAB_URL environment variable must be provided");
            std::process::exit(1);
        }
    }
    // Check if token is provided or GITLAB_ACCESS_TOKEN is set
    if args.token.is_none() {
        if let Ok(token) = std::env::var("GITLAB_ACCESS_TOKEN") {
            args.token = Some(token);
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
    // Clear title and description column if index is provided
    if args.title_index.is_some() {
        args.title_column = None;
    }
    if args.description_index.is_some() {
        args.description_column = None;
    }
    // Verify that title_index is provided if the csv file has no header
    if args.no_header && args.title_index.is_none() {
        eprintln!("title_index must be provided if the csv file has no header");
        std::process::exit(1);
    }
}

fn ask_user_for_token() -> Result<String, &'static str> {
    let mut buffer = String::new();
    println!("No token provided. Please enter your GitLab API token:");
    let token = match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => buffer.trim().to_string(),
        Err(_) => return Err("Could not read token"),
    };
    Ok(token)
}

fn args_to_parser(args: &Args, verbose: bool) -> issuefile::FileParser {
    let parser = issuefile::FileParser::new(
        args.file.as_ref().unwrap().to_path_buf(),
        args.separator.clone(),
        args.no_header.clone(),
        args.title_column.clone(),
        args.title_index,
        args.description_column.clone(),
        args.description_index,
        verbose,
    );
    parser
}

fn args_to_gitlabapi_request_client(
    args: &Args,
) -> Result<gitlabapi::GitLabApiRequest, &'static str> {
    let token: String = match args.token.as_ref() {
        Some(t) => t.clone(),
        None => {
            let token = loop {
                match ask_user_for_token() {
                    Ok(t) => break t,
                    Err(e) => eprintln!("{}", e),
                }
            };
            token
        }
    };
    let client = gitlabapi::GitLabApiRequest::new(
        args.url.as_ref().unwrap().as_str(),
        token,
        args.no_ssl_verify.unwrap(),
    );
    Ok(client)
}

fn get_valid_project_id(
    args: &Args,
    projects: Vec<gitlabapi::GitLabProject>,
) -> Result<u64, String> {
    // Check if the user provided project name or id
    if args.project_name.is_some() {
        let wanted_project_name = args.project_name.as_ref().unwrap();
        // It is possible that the user provided a project name,
        // for which there are multiple projects with the same name.
        // Check for name and namespace
        let mut matching_projects: Vec<u64> = Vec::new();
        projects.iter().for_each(|project| {
            if &project.name == wanted_project_name {
                matching_projects.push(project.id);
            }
            if &project.path_with_namespace == wanted_project_name {
                matching_projects.push(project.id);
            }
        });

        match matching_projects.len() {
            0 => {
                return Err(format!(
                    "No projects with name {} found",
                    wanted_project_name.clone()
                ))
            }
            1 => {
                return Ok(matching_projects[0]);
            }
            _ => {
                return Err(format!(
                    "Multiple projects with name {} found",
                    wanted_project_name.clone()
                ));
            }
        };
    } else {
        // args.project_id.is_some() is always true if we reach this point
        let wanted_project_id = args.project_id.unwrap();
        for project in projects {
            if project.id == wanted_project_id {
                return Ok(wanted_project_id);
            }
        }
        return Err(format!("No project with id {} found", wanted_project_id));
    }
}

fn main() {
    let mut args = Args::parse();
    verify_args(&mut args);
    // Translate args to file parser.
    // We dont need to check if the options are valid, because we already did that in verify_args
    // We make the parser mutable, because we might need to change the title and description column
    // if the user provided them
    let mut parser = args_to_parser(&args, args.verbose);
    // Attempt to read the file and extract the issues
    let fileissues = match parser.get_issues() {
        Ok(issues) => issues,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    if args.verbose {
        println!(
            "Found {} issues in the file {}:",
            fileissues.len(),
            args.file.as_ref().unwrap().to_str().unwrap()
        );
        fileissues
            .iter()
            .for_each(|issue| println!("\t{}", issue.to_string()))
    }
    // Exit if user only wanted to check the file
    if args.check {
        println!("File is valid, exiting because of --check flag...");
        std::process::exit(0);
    }

    // Create the gitlab api client
    let client = match args_to_gitlabapi_request_client(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    // Check if our token is valid by trying to get the available projects
    let projects = match client.get_projects() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    if args.verbose {
        println!("Found {} projects:", projects.len());
        projects
            .iter()
            .for_each(|project| println!("\t{}", project.to_string()))
    }
    // Verify that the project exists
    let project_id = match get_valid_project_id(&args, projects) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    if args.verbose {
        println!("Verified project id {}", project_id);
    }
    // If specified, verify that the assignee exists and is a member of the project
    let assignee_id: Option<u64> = None;
    if args.assignee.is_some() {
        let project_members = match client.get_members_of_project(project_id) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };
        if args.verbose {
            println!(
                "Found {} members of project {}",
                project_members.len(),
                project_id
            );
            project_members
                .iter()
                .for_each(|member| println!("\t{}", member.to_string()))
        }

        let our_assignee = args.assignee.as_ref().unwrap();
        if args.verbose {
            println!("Verifying that assignee {} exists...", our_assignee);
        }
        let mut assignee_exists = false;
        for member in project_members {
            if member.username == *our_assignee {
                assignee_exists = true;
                break;
            }
        }
        match assignee_exists {
            true => (),
            false => {
                eprintln!(
                    "The assignee {} does not exist or is not a member of the project with id {}",
                    our_assignee, project_id
                );
                std::process::exit(1);
            }
        }
    }

    // If specified, verify that the labels exist
    if args.labels.is_some() {
        let project_labels = match client.get_labels_of_project(project_id) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };
        if args.verbose {
            println!(
                "Found {} labels of project {}",
                project_labels.len(),
                project_id
            );
            project_labels
                .iter()
                .for_each(|label| println!("\t{}", label.to_string()))
        }
        let our_labels = args
            .labels
            .as_ref()
            .unwrap()
            .split(',')
            .collect::<Vec<&str>>();
        if args.verbose {
            println!("Verifying that labels {:?} exist...", our_labels);
        }
        for our_label in our_labels {
            let mut label_exists = false;
            for gitlab_label in &project_labels {
                if gitlab_label.name == *our_label {
                    label_exists = true;
                    break;
                }
            }
            match label_exists {
                true => (),
                false => {
                    eprintln!(
                        "The label {} does not exist in the project with id {}",
                        our_label, project_id
                    );
                    std::process::exit(1);
                }
            }
        }
    }
    // All checks passed, now we can create the issues
    for fileissue in fileissues {
        let issue =
            gitlabapi::GitLabProjectIssue::new(project_id, &fileissue, &args.labels, assignee_id);
        if args.verbose {
            println!(
                "Creating issue '{}' for project {}",
                issue.title, project_id
            );
        }
        match client.post_issue(&issue) {
            Ok(_) => println!("Created issue '{}'", issue.title),
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }
}
