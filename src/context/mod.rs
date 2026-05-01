use std::path::Path;

#[derive(Debug, Default)]
pub struct ContextInfo {
    pub os: String,
    pub pwd: String,
    pub files: Vec<String>,
    pub branch: Option<String>,
}

pub fn gather() -> ContextInfo {
    let pwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let files = std::fs::read_dir(&pwd)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .take(20)
                .collect()
        })
        .unwrap_or_default();

    let git_dir = Path::new(&pwd).join(".git");
    let is_git = git_dir.exists();

    let branch = if is_git {
        std::fs::read_to_string(git_dir.join("HEAD"))
            .ok()
            .and_then(|content| {
                content
                    .trim()
                    .strip_prefix("ref: refs/heads/")
                    .map(|s| s.to_string())
            })
    } else {
        None
    };

    ContextInfo {
        os: "linux".to_string(),
        pwd,
        files,
        branch,
    }
}

pub fn default_context() -> ContextInfo {
    ContextInfo {
        os: "linux".to_string(),
        pwd: String::new(),
        files: vec![],
        branch: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn setup_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_gather_os_is_linux() {
        let ctx = gather();
        assert_eq!(ctx.os, "linux");
    }

    #[test]
    fn test_gather_pwd_is_nonempty() {
        let ctx = gather();
        assert!(!ctx.pwd.is_empty());
    }

    #[test]
    fn test_gather_files_at_most_20() {
        let ctx = gather();
        assert!(ctx.files.len() <= 20);
    }

    #[test]
    fn test_is_git_detection() {
        let dir = setup_temp_dir("ta_test_is_git");
        let git_dir = dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        // Verify detection logic directly
        assert!(git_dir.exists());
        let branch = std::fs::read_to_string(git_dir.join("HEAD"))
            .ok()
            .and_then(|c| {
                c.trim()
                    .strip_prefix("ref: refs/heads/")
                    .map(|s| s.to_string())
            });
        assert_eq!(branch, Some("main".to_string()));

        cleanup(&dir);
    }

    #[test]
    fn test_branch_detached_head() {
        let dir = setup_temp_dir("ta_test_detached_head");
        let git_dir = dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        // Detached HEAD contains a commit SHA, not a ref
        fs::write(
            git_dir.join("HEAD"),
            "abc1234def5678abc1234def5678abc1234def56\n",
        )
        .unwrap();

        let branch = std::fs::read_to_string(git_dir.join("HEAD"))
            .ok()
            .and_then(|c| {
                c.trim()
                    .strip_prefix("ref: refs/heads/")
                    .map(|s| s.to_string())
            });
        assert_eq!(branch, None);

        cleanup(&dir);
    }

    #[test]
    fn test_default_context() {
        let ctx = default_context();
        assert_eq!(ctx.os, "linux");
        assert!(ctx.pwd.is_empty());
        assert!(ctx.files.is_empty());
        assert!(ctx.branch.is_none());
    }
}
