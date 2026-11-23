use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, State, Url};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct Updates(Mutex<Option<Update>>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    enabled: bool,
    endpoint: String,
    public_key: String,
}
#[derive(Serialize)]
pub struct Status {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

#[tauri::command]
pub async fn update_check(app: AppHandle, state: State<'_, Updates>) -> Result<Status, String> {
    let mut pending = state.0.lock().await;
    let config: Config =
        serde_json::from_str(include_str!("../updater.json")).map_err(|_| "更新配置格式无效")?;
    *pending = None;
    let mut status = Status {
        status: "unconfigured",
        version: None,
        notes: None,
    };
    if !config.enabled {
        return Ok(status);
    }
    let endpoint = Url::parse(&config.endpoint).map_err(|_| "更新源地址无效")?;
    if endpoint.scheme() != "https" || config.public_key.trim().is_empty() {
        return Err("更新服务需要 HTTPS 地址和验签公钥".into());
    }
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| e.to_string())?
        .pubkey(config.public_key)
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    *pending = updater
        .check()
        .await
        .map_err(|e| format!("检查更新失败：{e}"))?;
    if let Some(update) = pending.as_ref() {
        if update.download_url.scheme() != "https" {
            *pending = None;
            return Err("更新包下载地址必须使用 HTTPS".into());
        }
        status.status = "available";
        status.version = Some(update.version.clone());
        status.notes = update.body.clone();
    } else {
        status.status = "current";
    }
    Ok(status)
}
#[tauri::command]
pub async fn update_install(state: State<'_, Updates>) -> Result<(), String> {
    let pending = state.0.lock().await;
    let update = pending.as_ref().ok_or("请先检查可用更新")?;
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("安装更新失败：{e}"))
}
