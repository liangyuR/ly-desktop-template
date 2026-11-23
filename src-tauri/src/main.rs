#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod licensing;
mod logging;
mod mcp;
mod projects;
mod settings;
mod updates;

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::Manager;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: String,
    version: String,
    commit: String,
    built_at: String,
    data_dir: PathBuf,
    log_dir: PathBuf,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeConfig {
    log_retention_days: u16,
}

#[tauri::command]
fn app_info(state: tauri::State<AppInfo>) -> AppInfo {
    state.inner().clone()
}
#[tauri::command]
fn startup_ready(window: tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())
}

fn run() -> tauri::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = app
                .path()
                .app_local_data_dir()?
                .join(if cfg!(debug_assertions) {
                    "development"
                } else {
                    "production"
                });
            fs::create_dir_all(&root)?;
            let lease = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(root.join(".instance.lock"))?;
            lease.try_lock().map_err(|e| {
                format!(
                    "无法独占数据目录 {}，请检查应用是否已打开：{e}",
                    root.display()
                )
            })?;
            app.manage(lease);
            let log_dir = root.join("logs");
            let config: RuntimeConfig = serde_json::from_str(include_str!("../runtime.json"))?;
            app.manage(logging::initialize(
                log_dir.clone(),
                config.log_retention_days,
            )?);
            tracing::info!("应用启动");
            let preferences = settings::Preferences::open(&root)?;
            let server = mcp::Mcp::default();
            if let Err(e) = tauri::async_runtime::block_on(server.restore(&preferences)) {
                tracing::warn!(reason=%e,"无法恢复 MCP 监听");
            }
            app.manage(server);
            app.manage(preferences);
            app.manage(projects::Projects::open(&root)?);
            app.manage(updates::Updates::default());
            licensing::initialize(root.clone());
            let built_at = env!("APP_BUILD_SECONDS")
                .parse()
                .ok()
                .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();
            app.manage(AppInfo {
                name: app
                    .config()
                    .product_name
                    .clone()
                    .unwrap_or_else(|| app.package_info().name.clone()),
                version: app.package_info().version.to_string(),
                commit: env!("APP_COMMIT").into(),
                built_at,
                data_dir: root,
                log_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            startup_ready,
            settings::get_settings,
            settings::set_theme,
            logging::open_current_log,
            mcp::mcp_status,
            mcp::mcp_configure,
            updates::update_check,
            updates::update_install,
            licensing::license_status,
            licensing::license_activate,
            licensing::license_sync,
            projects::projects_list,
            projects::projects_create,
            projects::projects_rename,
            projects::projects_switch,
            projects::projects_delete,
            projects::projects_copy,
            projects::projects_import,
            projects::projects_export
        ])
        .run(tauri::generate_context!())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("启动失败：{e}");
        rfd::MessageDialog::new()
            .set_title("无法启动应用")
            .set_description(e.to_string())
            .set_level(rfd::MessageLevel::Error)
            .show();
        std::process::exit(1);
    }
}
