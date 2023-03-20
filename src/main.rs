use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Path to the file to upload. Required.
    #[arg(short, long, value_name = "FILE", required = true)]
    file: Option<std::path::PathBuf>,
    // URL of the GitLab instance, e.g. https://gitlab.com. Defaults to https://localhost.
    #[arg(short, long, default_value = "https://localhost")]
    url: Option<String>,
    // GitLab API token. If not provided, the GITLAB_ACCESS_TOKEN environment variable is used.
    // If neither is provided, the program will prompt for a token.
    #[arg(short, long)]
    token: Option<String>,
    // Name of the gitlab project to upload to. Required if project_id is not provided.
    #[arg(short, long)]
    project_name: Option<String>,
    // ID of the gitlab project to upload to. Required if project_name is not provided.
    #[arg(long)]
    project_id: Option<u32>,
    // Comma separated list of labels to add to the issue. None are added by default.
    #[arg(short, long)]
    labels: Option<String>,
    // Assignee username to add to the issue. None are added by default.
    #[arg(short, long)]
    assignee: Option<String>,
    // Should we disable SSL verification for requests to gitlab? Defaults to false.
    #[arg(short, long, default_value = "false")]
    no_ssl_verify: Option<bool>,
    // Check if the file can be used to extract gitlab tasks. No upload is performed. Defaults to false.
    #[arg(short, long, default_value = "false")]
    check: Option<bool>,
    // Verbose output. Defaults to false.
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
    }
    // Verify that either url is provided or GITLAB_URL is set
    if args.url.is_none() {
        if let Ok(url) = std::env::var("GITLAB_URL") {
            args.url = Some(url);
        } else {
            eprintln!("Either url by argument or GITLAB_URL environment variable must be provided");
            std::process::exit(1);
        }
    }
    // Verify that either token is provided or GITLAB_ACCESS_TOKEN is set
    if args.token.is_none() {
        if let Ok(token) = std::env::var("GITLAB_ACCESS_TOKEN") {
            args.token = Some(token);
        } else {
            eprintln!("Either token by argument or GITLAB_ACCESS_TOKEN environment variable must be provided");
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

fn main() {
    let mut args = Args::parse();
    verify_args(&mut args);
    if let Some(file) = args.file {
        println!("File: {:?}", file);
    }
    println!("Hello, world!");
}
