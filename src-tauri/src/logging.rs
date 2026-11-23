use chrono::{Datelike, Local, NaiveDate, NaiveDateTime};
use flexi_logger::{Age, Cleanup, Criterion, FileSpec, Naming, writers::FileLogWriter};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicI32, Ordering},
};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use tracing_subscriber::prelude::*;

pub const CURRENT: &str = "desktop_rCURRENT.txt";

fn archive_date(name: &str) -> Option<NaiveDate> {
    let stem = name.strip_prefix("desktop_r")?.strip_suffix(".txt")?;
    let timestamp = match stem.split_once(".restart-") {
        Some((ts, n)) if n.len() >= 4 && n.bytes().all(|b| b.is_ascii_digit()) => ts,
        Some(_) => return None,
        None => stem,
    };
    NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d_%H-%M-%S")
        .ok()
        .map(|v| v.date())
}

fn prune(dir: &Path, today: NaiveDate, days: u16) -> std::io::Result<()> {
    let earliest = today - chrono::Days::new(u64::from(days - 1));
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .file_name()
                .to_str()
                .and_then(archive_date)
                .is_some_and(|d| d < earliest)
        {
            if let Err(e) = fs::remove_file(entry.path()) {
                eprintln!("归档清理失败：{e}");
            }
        }
    }
    Ok(())
}

pub fn initialize(
    dir: PathBuf,
    days: u16,
) -> Result<impl Send + Sync + 'static, Box<dyn std::error::Error>> {
    if !(1..=365).contains(&days) {
        return Err("日志保留天数应为 1 至 365".into());
    }
    let spec = FileSpec::default()
        .directory(&dir)
        .basename("desktop")
        .suffix("txt")
        .suppress_timestamp();
    let (writer, handle) = FileLogWriter::builder(spec)
        .append()
        .rotate(
            Criterion::AgeOrSize(Age::Day, 10 * 1024 * 1024),
            Naming::Timestamps,
            Cleanup::Never,
        )
        .try_build_with_handle()?;
    let last_day = AtomicI32::new(0);
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(move || {
            let today = Local::now().date_naive();
            if last_day.swap(today.num_days_from_ce(), Ordering::Relaxed)
                != today.num_days_from_ce()
            {
                if let Err(e) = prune(&dir, today, days) {
                    eprintln!("日志清理失败：{e}");
                }
            }
            writer.clone()
        });
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(layer)
        .try_init()?;
    Ok(handle)
}

#[tauri::command]
pub fn open_current_log(app: tauri::AppHandle) -> Result<(), String> {
    let file = app.state::<crate::AppInfo>().log_dir.join(CURRENT);
    app.opener()
        .open_path(file.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("无法打开 {}：{e}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_segments_before_local_cutoff_are_pruned_but_other_files_survive() {
        let dir = tempfile::tempdir().unwrap();
        let names = [
            "desktop_r2026-10-01_23-59-59.txt",
            "desktop_r2026-10-01_23-59-59.restart-0001.txt",
            "desktop_r2026-10-02_00-00-00.txt",
            CURRENT,
            "unrelated.txt",
        ];
        for n in names {
            fs::write(dir.path().join(n), "log").unwrap();
        }
        prune(dir.path(), NaiveDate::from_ymd_opt(2026, 10, 3).unwrap(), 2).unwrap();
        assert_eq!(
            names.map(|n| dir.path().join(n).exists()),
            [false, false, true, true, true]
        );
    }
}
