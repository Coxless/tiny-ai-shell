#[derive(Debug, Default)]
pub struct ContextInfo {
    pub os: String,
    pub pwd: String,
    pub files: Vec<String>,
    pub is_git: bool,
    pub branch: Option<String>,
}

pub fn gather() -> ContextInfo {
    todo!("implement in step 3")
}

pub fn default_context() -> ContextInfo {
    ContextInfo {
        os: "linux".to_string(),
        pwd: String::new(),
        files: vec![],
        is_git: false,
        branch: None,
    }
}
