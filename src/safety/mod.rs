#[derive(Debug)]
pub struct DangerResult {
    pub is_dangerous: bool,
    pub message: Option<String>,
}

pub fn check(_command: &str) -> DangerResult {
    todo!("implement in step 4")
}
