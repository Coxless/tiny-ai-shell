pub mod color;

#[derive(Debug)]
pub enum Action {
    Execute,
    Cancel,
    Copy,
    Explain,
    Rewrite,
}

pub fn prompt_action(_command: &str) -> anyhow::Result<Action> {
    todo!("implement in step 5")
}
