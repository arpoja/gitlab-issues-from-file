use log::{debug, error, info, warn};
use reqwest;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

use crate::issuefile::IssueFromFile;

pub struct GitLabProjectMember {
    pub id: u64,
    pub username: String,
    name: String,
}
impl fmt::Display for GitLabProjectMember {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {} ({})", self.id, self.username, self.name)
    }
}
pub struct GitLabProjectLabel {
    id: u64,
    pub name: String,
}
impl fmt::Display for GitLabProjectLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.id, self.name)
    }
}

pub struct GitLabProject {
    pub id: u64,
    pub name: String,
    pub path_with_namespace: String,
    members: Option<Vec<GitLabProjectMember>>,
    labels: Option<Vec<GitLabProjectLabel>>,
}
impl fmt::Display for GitLabProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {} ({})",
            self.id, self.name, self.path_with_namespace
        )
    }
}

pub struct GitLabApiRequest {
    base_url: String,
    headers: reqwest::header::HeaderMap,
    client: reqwest::blocking::Client,
}
impl GitLabApiRequest {
    pub fn new(base_url: &str, token: String, no_ssl_verify: bool) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("PRIVATE-TOKEN", token.parse().unwrap());
        let client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(no_ssl_verify)
            .build()
            .unwrap();
        Self {
            base_url: format!("{}/api/v4", base_url.to_string()),
            headers,
            client,
        }
    }
    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, &'static str> {
        // Create the url, if the path is /projects, the url will be <GITLAB_URL>/api/v4/projects
        // Check if the first character of the path is a /, if it is, remove it
        let path = if path.chars().nth(0).unwrap() == '/' {
            path[1..].to_string()
        } else {
            path.to_string()
        };
        let url = format!("{}/{}", self.base_url, path);
        debug!("Sending GET request to {}", url);
        let response = match self.client.get(&url).headers(self.headers.clone()).send() {
            Ok(response) => response,
            Err(_) => return Err("Failed to send request"),
        };
        debug!("Response rc: {}", &response.status());
        // Check if the response was successful
        if !response.status().is_success() {
            debug!("Unsuccesful response body: {}", &response.text().unwrap());
            return Err("Request was not successful");
        }
        Ok(response)
    }
    fn post(
        &self,
        path: &str,
        body: &HashMap<&str, String>,
    ) -> Result<reqwest::blocking::Response, &'static str> {
        // Create the url, if the path is /projects, the url will be <GITLAB_URL>/api/v4/projects
        // Check if the first character of the path is a /, if it is, remove it
        let path = if path.chars().nth(0).unwrap() == '/' {
            path[1..].to_string()
        } else {
            path.to_string()
        };
        let url = format!("{}/{}", self.base_url, path);
        debug!("Sending POST request to {}", url);
        let response = match self
            .client
            .post(&url)
            .headers(self.headers.clone())
            .json(&body)
            .send()
        {
            Ok(response) => response,
            Err(_) => return Err("Failed to send request"),
        };
        debug!("Response rc: {}", &response.status());
        // Check if the response was successful
        if !response.status().is_success() {
            debug!("Unsuccesful response body: {}", &response.text().unwrap());
            return Err("Request was not successful");
        }
        Ok(response)
    }
    pub fn get_projects(&self) -> Result<Vec<GitLabProject>, &'static str> {
        debug!("Getting projects from GitLab (GET /projects)");
        let path = "projects";
        let response = match self.get(path) {
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
            Err(e) => {
                error!("Error parsing projects: {}", e);
                return Err("Failed to parse response");
            }
        };
        let mut projects: Vec<GitLabProject> = Vec::new();
        // Turn the response into a vector of structs
        for project in projects_array {
            let p = GitLabProject {
                id: project["id"].as_u64().unwrap(),
                name: project["name"].as_str().unwrap().to_string(),
                path_with_namespace: project["path_with_namespace"].as_str().unwrap().to_string(),
                members: None,
                labels: None,
            };
            projects.push(p);
        }
        Ok(projects)
    }
    pub fn get_members_of_project(
        &self,
        project_id: u64,
    ) -> Result<Vec<GitLabProjectMember>, &'static str> {
        let path = format!("projects/{}/members", project_id);
        let response = match self.get(&path) {
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
            Err(e) => {
                error!("Error parsing members {}", e);
                return Err("Failed to parse response");
            }
        };
        let mut members: Vec<GitLabProjectMember> = Vec::new();
        for member in members_array {
            let m = GitLabProjectMember {
                id: member["id"].as_u64().unwrap(),
                username: member["username"].as_str().unwrap().to_string(),
                name: member["name"].as_str().unwrap().to_string(),
            };
            members.push(m);
        }
        Ok(members)
    }

    pub fn get_labels_of_project(
        &self,
        project_id: u64,
    ) -> Result<Vec<GitLabProjectLabel>, &'static str> {
        let path = format!("projects/{}/labels", project_id);
        let response = match self.get(&path) {
            Ok(response) => response,
            Err(_) => return Err("Failed to send request"),
        };
        // Check if the response was successful
        if !response.status().is_success() {
            return Err("Request was not successful");
        }
        // Parse the response with serde before turning the important info into a vector of structs
        let labels_array: Vec<serde_json::Value> = match response.json() {
            Ok(labels) => labels,
            Err(e) => {
                error!("Error parsing labels {}", e);
                return Err("Failed to parse response");
            }
        };
        let mut labels: Vec<GitLabProjectLabel> = Vec::new();
        for label in labels_array {
            let l = GitLabProjectLabel {
                id: label["id"].as_u64().unwrap(),
                name: label["name"].as_str().unwrap().to_string(),
            };
            labels.push(l);
        }
        Ok(labels)
    }

    pub fn get_projects_with_members_and_labels(&self) -> Result<Vec<GitLabProject>, &'static str> {
        let mut projects = match self.get_projects() {
            Ok(projects) => projects,
            Err(_) => return Err("Failed to get projects"),
        };
        for project in &mut projects {
            let members = match self.get_members_of_project(project.id) {
                Ok(members) => members,
                Err(_) => return Err("Failed to get members of project"),
            };
            let labels = match self.get_labels_of_project(project.id) {
                Ok(labels) => labels,
                Err(_) => return Err("Failed to get labels of project"),
            };
            project.members = Some(members);
            project.labels = Some(labels);
        }
        Ok(projects)
    }

    pub fn post_issue(&self, issue: &GitLabProjectIssue) -> Result<(), &'static str> {
        let body = issue.create_issue_body();
        let path = format!("projects/{}/issues", issue.project_id);
        let response = match self.post(&path, &body.unwrap()) {
            Ok(response) => response,
            Err(_) => return Err("Failed to send request"),
        };
        // Check if the response was successful
        if !response.status().is_success() {
            return Err("Request was not successful");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct GitLabProjectIssue {
    id: Uuid,
    project_id: u64,
    pub title: String,
    description: Option<String>,
    labels: Option<String>,
    assignee_id: Option<u64>,
}
impl GitLabProjectIssue {
    pub fn new(
        project_id: u64,
        issue: &IssueFromFile,
        labels: &Option<String>,
        assignee_id: Option<u64>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            title: issue.title.clone(),
            description: issue.description.clone(),
            labels: labels.clone(),
            assignee_id: assignee_id,
        }
    }
    fn create_issue_body(&self) -> Result<HashMap<&str, String>, &'static str> {
        let mut body = HashMap::new();
        body.insert("id", self.id.to_string());
        body.insert("title", self.title.clone());
        if let Some(description) = &self.description {
            body.insert("description", description.clone());
        }
        if let Some(labels) = &self.labels {
            body.insert("labels", labels.clone());
        }
        if let Some(assignee_id) = &self.assignee_id {
            body.insert("assignee_id", assignee_id.to_string());
        }
        Ok(body)
    }
}
