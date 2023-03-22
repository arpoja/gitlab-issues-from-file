use csv::ReaderBuilder;
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
    verbose: bool,
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
        verbose: bool,
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
            verbose: verbose,
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
        if self.verbose {
            println!("Parsing csv file with options: {:#?}", self);
        }
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
            if self.verbose {
                println!("CSV file has headers {:?}", headers);
            }
            // Get title column index if title_column is set by name
            if self.title_column.is_some() {
                if self.verbose {
                    println!(
                        "title_column is set to '{}', trying to find column index...",
                        self.title_column.as_ref().unwrap()
                    );
                }
                if let Some(title_column_index) = headers
                    .iter()
                    .position(|x| x == self.title_column.as_ref().unwrap().as_str())
                {
                    self.title_column_index = Some(title_column_index);
                    if self.verbose {
                        println!(
                            "Found title_column_index: {}",
                            self.title_column_index.unwrap()
                        );
                    }
                } else {
                    return Err(format!(
                        "Could not find column with name '{}'",
                        self.title_column.as_ref().unwrap()
                    ));
                }
            }
            // Get description column index if description_column is set by name
            if self.description_column.is_some() {
                if self.verbose {
                    println!(
                        "description_column is set to '{}', trying to find column index...",
                        self.description_column.as_ref().unwrap()
                    );
                }
                if let Some(description_column_index) = headers
                    .iter()
                    .position(|x| x == self.description_column.as_ref().unwrap().as_str())
                {
                    self.description_column_index = Some(description_column_index);
                    if self.verbose {
                        println!(
                            "Found description_column_index: {}",
                            self.description_column_index.unwrap()
                        );
                    }
                } else {
                    return Err(format!(
                        "Could not find column with name '{}'",
                        self.description_column.as_ref().unwrap()
                    ));
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
        if self.verbose {
            println!("Column indeces set to valid values: {:#?}", self);
        }
        // We now have valid title_column_index and if set, description_column_index as well
        // Start building issues
        let mut issues: Vec<IssueFromFile> = Vec::new();
        // Step through the records
        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => {
                    if self.verbose {
                        println!("Error reading record: {:#?}", result);
                    }
                    return Err(String::from("Could not read record"));
                }
            };
            // Get title
            let title = match record.get(self.title_column_index.unwrap()) {
                Some(t) => t,
                None => return Err(String::from("Could not get title")),
            };
            // Get description
            let description = match self.description_column_index {
                Some(i) => match record.get(i) {
                    Some(d) => d,
                    None => return Err(String::from("Could not get description")),
                },
                None => "",
            };
            // Build issue and push it to issues
            let issue = IssueFromFile {
                title: title.to_string(),
                description: Some(description.to_string()),
            };
            issues.push(issue);
        }
        //
        Ok(issues)
    }
    fn json_to_issues(&self) -> Result<Vec<IssueFromFile>, String> {
        Err(String::from("Not implemented"))
    }

    pub fn verify_supported_file_type(file: &PathBuf) -> Result<(), &'static str> {
        let file_type = file.extension().unwrap();
        match file_type.to_ascii_lowercase().to_str().unwrap() {
            "csv" => Ok(()),
            "json" => Err("Not implemented currently"),
            _ => Err("Unsupported file type"),
        }
    }
}
