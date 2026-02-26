use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize)]
pub struct CrashInfo {
    pub timestamp: String,
    pub error: String,
}

pub fn record_crash(error: &str, workspace_dir: &Path) {
    let crash_file = workspace_dir.join("last_crash.json");
    let info = CrashInfo {
        timestamp: Utc::now().to_rfc3339(),
        error: error.to_string(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&info) {
        let _ = fs::write(crash_file, json);
    }
}

pub fn consume_crash(workspace_dir: &Path) -> Option<CrashInfo> {
    let crash_file = workspace_dir.join("last_crash.json");
    if crash_file.exists() {
        if let Ok(content) = fs::read_to_string(&crash_file) {
            let _ = fs::remove_file(&crash_file);
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

pub fn report_task(title: &str, detail: &str, workspace_dir: &Path) {
    let tasks_file = workspace_dir.join("RESILIENCE_TASKS.md");
    let timestamp = Utc::now().to_rfc3339();
    let entry = format!("\n## [{}] {}\n\n- **Time**: {}\n- **Detail**: {}\n\n---",
        if detail.contains("fatal") { "FATAL" } else { "RECOVERY" },
        title,
        timestamp,
        detail
    );

    let mut content = if tasks_file.exists() {
        fs::read_to_string(&tasks_file).unwrap_or_default()
    } else {
        "# Resilience Tasks\n\nThis file tracks critical failures and auto-recovery events.\n".to_string()
    };

    content.push_str(&entry);
    let _ = fs::write(tasks_file, content);
}
