use csv::ReaderBuilder;
use log::{debug, error, info, warn};
use std::path::PathBuf;
pub struct IssueFromFile {
    pub title: String,
    pub description: Option<String>,
}
impl IssueFromFile {
    pub fn to_string(&self) -> String {
        format!(
            "Title: {}, Description: {}",
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
    title_column: Option<String>,
    title_column_index: Option<usize>,
    description_column: Option<String>,
    description_column_index: Option<usize>,
}
impl FileParser {
    pub fn new(
        file: PathBuf,
        separator: Option<char>,
        no_header: bool,
        title_column: Option<String>,
        title_column_index: Option<usize>,
        description_column: Option<String>,
        description_column_index: Option<usize>,
    ) -> FileParser {
        let file_extension = file.extension().unwrap().to_str().unwrap().to_lowercase();
        FileParser {
            file: file.clone(),
            file_extension: file_extension,
            separator: separator,
            no_header: no_header,
            title_column: title_column.clone(),
            title_column_index: title_column_index,
            description_column: description_column.clone(),
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
            if self.title_column.is_some() {
                debug!(
                    "User specified title_column: '{}', trying to find column index...",
                    self.title_column.as_ref().unwrap()
                );
                // Get index of title_column, match any case
                headers
                    .iter()
                    .position(|x| {
                        x.to_lowercase()
                            == self.title_column.as_ref().unwrap().to_lowercase().as_str()
                    })
                    .map(|i| self.title_column_index = Some(i));
                match self.title_column_index {
                    Some(i) => debug!("Found title_column_index: {}", i),
                    None => {
                        return Err(format!(
                            "Could not find column with name '{}'",
                            self.title_column.as_ref().unwrap()
                        ))
                    }
                }
            }
            // Get description column index if description_column is set by name
            if self.description_column.is_some() {
                debug!(
                    "User specified description_column: '{}', trying to find column index...",
                    self.description_column.as_ref().unwrap()
                );
                // Get index of description_column, match any case
                headers
                    .iter()
                    .position(|x| {
                        x.to_lowercase()
                            == self
                                .description_column
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
                            self.description_column.as_ref().unwrap()
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

        Err(String::from("Not implemented"))
    }
}
