use csv::ReaderBuilder;
use log::{debug, error, info, warn};
use std::fmt;
use std::path::PathBuf;
pub struct IssueFromFile {
    pub title: String,
    pub description: Option<String>,
}
impl fmt::Display for IssueFromFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Title: '{}', Description: '{}'",
            self.title,
            self.description.as_ref().unwrap_or(&"".to_string())
        )
    }
}

pub const SUPPORTED_FILE_TYPES: [&str; 2] = ["csv", "json"];
#[derive(Debug)]
pub struct FileParser {
    file: PathBuf,
    file_extension: String,
    separator: Option<char>,
    no_header: bool,
    title_key: Option<String>,
    title_column_index: Option<usize>,
    description_key: Option<String>,
    description_column_index: Option<usize>,
}
impl FileParser {
    pub fn new(
        file: PathBuf,
        separator: Option<char>,
        no_header: bool,
        title_key: Option<String>,
        title_column_index: Option<usize>,
        description_key: Option<String>,
        description_column_index: Option<usize>,
    ) -> FileParser {
        let file_extension = file.extension().unwrap().to_str().unwrap().to_lowercase();
        FileParser {
            file: file.clone(),
            file_extension: file_extension,
            separator: separator,
            no_header: no_header,
            title_key: title_key.clone(),
            title_column_index: title_column_index,
            description_key: description_key.clone(),
            description_column_index: description_column_index,
        }
    }
    pub fn get_issues(&mut self) -> Result<Vec<IssueFromFile>, String> {
        match self.file_extension.as_str() {
            "csv" => self.csv_to_issues(),
            "json" => self.json_to_issues(),
            _ => return Err(String::from("Unsupported file type")),
        }
    }
    fn csv_to_issues(&mut self) -> Result<Vec<IssueFromFile>, String> {
        debug!("Parsing csv file with options: {:#?}", self);
        // Open csv reader
        let mut reader = ReaderBuilder::new()
            .has_headers(!self.no_header)
            .delimiter(self.separator.unwrap().to_string().as_bytes()[0])
            .from_path(&self.file)
            .unwrap();
        // Get title and description column index
        if !self.no_header {
            let headers = match reader.headers() {
                Ok(h) => h,
                Err(_) => return Err(String::from("Could not read headers")),
            };
            debug!("CSV file has headers {:?}", headers);
            // Get title column index if title_column is set by name
            if self.title_key.is_some() {
                debug!(
                    "User specified title_column: '{}', trying to find column index...",
                    self.title_key.as_ref().unwrap()
                );
                // Get index of title_column, match any case
                headers
                    .iter()
                    .position(|x| {
                        x.to_lowercase() == self.title_key.as_ref().unwrap().to_lowercase().as_str()
                    })
                    .map(|i| self.title_column_index = Some(i));
                match self.title_column_index {
                    Some(i) => debug!("Found title_column_index: {}", i),
                    None => {
                        return Err(format!(
                            "Could not find column with name '{}'",
                            self.title_key.as_ref().unwrap()
                        ))
                    }
                }
            }
            // Get description column index if description_column is set by name
            if self.description_key.is_some() {
                debug!(
                    "User specified description_column: '{}', trying to find column index...",
                    self.description_key.as_ref().unwrap()
                );
                // Get index of description_column, match any case
                headers
                    .iter()
                    .position(|x| {
                        x.to_lowercase()
                            == self
                                .description_key
                                .as_ref()
                                .unwrap()
                                .to_lowercase()
                                .as_str()
                    })
                    .map(|i| self.description_column_index = Some(i));
                match self.description_column_index {
                    Some(i) => debug!("Found description_column_index: {}", i),
                    None => {
                        return Err(format!(
                            "Could not find column with name '{}'",
                            self.description_key.as_ref().unwrap()
                        ))
                    }
                }
            }
        }
        // Are title_column_index and description_column_index within bounds?
        // We dont need to check if title_column_index is Some, because we would have returned already
        if self.title_column_index.unwrap() >= reader.headers().unwrap().len() {
            return Err(String::from("title_column_index is out of bounds"));
        }
        // We need to check if description_column_index is Some, because it is optional
        if self.description_column_index.is_some() {
            if self.description_column_index.unwrap() >= reader.headers().unwrap().len() {
                return Err(String::from("description_column_index is out of bounds"));
            }
        }
        // We now have valid title_column_index and if set, description_column_index as well
        // Start building issues
        let mut issues: Vec<IssueFromFile> = Vec::new();
        // Step through the records
        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => {
                    error!("Error reading record: {:#?}", result);
                    return Err(String::from("Could not read record"));
                }
            };
            // Get title
            let title = match record.get(self.title_column_index.unwrap()) {
                Some(t) => t.to_string(),
                None => return Err(String::from("Could not get title")),
            };
            // Get description
            let description = match self.description_column_index {
                Some(i) => match record.get(i) {
                    Some(d) => Some(d.to_string()),
                    None => None,
                },
                None => None,
            };
            // Build issue and push it to issues
            let issue = IssueFromFile {
                title: title,
                description: description,
            };
            issues.push(issue);
        }
        //
        Ok(issues)
    }
    fn json_to_issues(&self) -> Result<Vec<IssueFromFile>, String> {
        debug!("Parsing json file with options: {:#?}", self);
        let mut issues: Vec<IssueFromFile> = Vec::new();
        // Read json file to string and parse it
        let mut contents = match std::fs::read_to_string(&self.file) {
            Ok(c) => c,
            Err(e) => return Err(format!("Could not read file: {}", e)),
        };
        let data: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(j) => j,
            Err(e) => return Err(format!("Could not parse json: {}", e)),
        };
        // Check if data is an array of objects
        debug!("Json data: {:#?}", data);
        if data.is_array() {
            for item in data.as_array().unwrap() {
                debug!("Item: {:#?}", item);
                if item.is_object() {
                    let issue = match self.serde_object_to_issue(item.as_object().unwrap()) {
                        Ok(i) => i,
                        Err(e) => return Err(e),
                    };
                    issues.push(issue);
                } else {
                    return Err(String::from(
                        "Json data is not of a format that can be parsed",
                    ));
                }
            }
        } else if data.is_object() {
            let issue = match self.serde_object_to_issue(data.as_object().unwrap()) {
                Ok(i) => i,
                Err(e) => return Err(e),
            };
            issues.push(issue);
        } else {
            return Err(String::from(
                "Json data is not of a format that can be parsed",
            ));
        }

        Ok(issues)
    }
    fn serde_object_to_issue(
        &self,
        data: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<IssueFromFile, String> {
        // Loop through the keys and check if they are valid
        let mut title: String = String::new();
        let mut description: Option<String> = None;
        let our_title_name = self.title_key.as_ref().unwrap().to_lowercase();
        let our_description_name = self.description_key.as_ref().unwrap().to_lowercase();
        for (key, value) in data {
            let key = key.to_lowercase();
            if key == our_title_name {
                if value.is_string() {
                    title = value.as_str().unwrap().to_string();
                } else {
                    return Err(String::from("Title is not a string"));
                }
            } else if key == our_description_name {
                if value.is_string() {
                    description = Some(value.as_str().unwrap().to_string());
                } else {
                    return Err(String::from("Description is not a string"));
                }
            }
        }
        Ok(IssueFromFile {
            title: title,
            description: description,
        })
    }
}
