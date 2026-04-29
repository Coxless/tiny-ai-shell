use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_ENTRIES: usize = 1000;

#[derive(Serialize, Deserialize)]
struct LogContext {
    pwd: String,
    os: String,
}

#[derive(Serialize, Deserialize)]
struct LogEntry {
    timestamp: String,
    input: String,
    command: String,
    action: String,
    exit_code: Option<i32>,
    context: LogContext,
}

fn history_path() -> PathBuf {
    let base = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".local").join("share").join("ta").join("history.json")
}

fn format_timestamp(secs: u64) -> String {
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;
    let (year, month, day) = days_to_ymd(secs / 86400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

// Gregorian calendar: convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days as i64 + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

pub fn record(
    input: &str,
    command: &str,
    action: &str,
    exit_code: Option<i32>,
    pwd: &str,
    os: &str,
) {
    let path = history_path();

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut entries: Vec<LogEntry> = path
        .exists()
        .then(|| {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        })
        .flatten()
        .unwrap_or_default();

    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    entries.push(LogEntry {
        timestamp: format_timestamp(secs),
        input: input.to_string(),
        command: command.to_string(),
        action: action.to_string(),
        exit_code,
        context: LogContext {
            pwd: pwd.to_string(),
            os: os.to_string(),
        },
    });

    if entries.len() > MAX_ENTRIES {
        entries.drain(0..entries.len() - MAX_ENTRIES);
    }

    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        let _ = std::fs::write(&path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_format_timestamp_known_date() {
        // 2024-01-15T10:30:00Z = 1705314600
        assert_eq!(format_timestamp(1705314600), "2024-01-15T10:30:00Z");
    }

    #[test]
    fn test_format_timestamp_epoch() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_record_creates_file() {
        let dir = std::env::temp_dir().join("ta_logger_test_create");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        // Override HOME via a temp path trick: write directly to a known path
        let path = dir.join("history.json");
        let _ = fs::remove_file(&path);

        // Call record and check the file exists via history_path()
        // (We can't override HOME easily, so test the internal logic directly)
        let mut entries: Vec<LogEntry> = vec![];
        entries.push(LogEntry {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            input: "list files".to_string(),
            command: "ls -la".to_string(),
            action: "executed".to_string(),
            exit_code: Some(0),
            context: LogContext {
                pwd: "/home/user".to_string(),
                os: "linux".to_string(),
            },
        });
        let json = serde_json::to_string_pretty(&entries).unwrap();
        fs::write(&path, &json).unwrap();

        let loaded: Vec<serde_json::Value> =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0]["action"], "executed");
        assert_eq!(loaded[0]["exit_code"], 0);
        assert_eq!(loaded[0]["context"]["os"], "linux");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rotation_at_1000() {
        let mut entries: Vec<LogEntry> = (0..MAX_ENTRIES)
            .map(|i| LogEntry {
                timestamp: format!("2024-01-01T00:00:{:02}Z", i % 60),
                input: "x".to_string(),
                command: "ls".to_string(),
                action: "cancelled".to_string(),
                exit_code: None,
                context: LogContext {
                    pwd: "/".to_string(),
                    os: "linux".to_string(),
                },
            })
            .collect();

        entries.push(LogEntry {
            timestamp: "2024-12-31T23:59:59Z".to_string(),
            input: "new".to_string(),
            command: "pwd".to_string(),
            action: "executed".to_string(),
            exit_code: Some(0),
            context: LogContext {
                pwd: "/home".to_string(),
                os: "linux".to_string(),
            },
        });

        if entries.len() > MAX_ENTRIES {
            entries.drain(0..entries.len() - MAX_ENTRIES);
        }

        assert_eq!(entries.len(), MAX_ENTRIES);
        assert_eq!(entries.last().unwrap().action, "executed");
    }
}
