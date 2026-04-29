pub struct Colors {
    enabled: bool,
}

impl Colors {
    pub fn new() -> Self {
        Colors {
            enabled: std::env::var("NO_COLOR").is_err(),
        }
    }

    pub fn cyan(&self, s: &str) -> String {
        if self.enabled {
            format!("\x1b[36m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    pub fn red(&self, s: &str) -> String {
        if self.enabled {
            format!("\x1b[31m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
}
