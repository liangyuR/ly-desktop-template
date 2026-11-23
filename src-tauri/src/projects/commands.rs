use super::{Projects, Snapshot};
use std::io;
use tauri::State;

async fn blocking<T: Send + 'static>(
    projects: &Projects,
    work: impl FnOnce(Projects) -> io::Result<T> + Send + 'static,
) -> Result<T, String> {
    let projects = projects.clone();
    tauri::async_runtime::spawn_blocking(move || work(projects))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn projects_list(state: State<'_, Projects>) -> Result<Snapshot, String> {
    state.snapshot().map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn projects_create(
    state: State<'_, Projects>,
    name: String,
    description: String,
) -> Result<Snapshot, String> {
    blocking(&state, move |p| p.create(name, description)).await
}

#[tauri::command]
pub async fn projects_switch(state: State<'_, Projects>, id: String) -> Result<Snapshot, String> {
    blocking(&state, move |p| p.switch(&id)).await
}

#[tauri::command]
pub async fn projects_rename(
    state: State<'_, Projects>,
    id: String,
    name: String,
    description: String,
) -> Result<Snapshot, String> {
    blocking(&state, move |p| p.rename(&id, name, description)).await
}

#[tauri::command]
pub async fn projects_delete(state: State<'_, Projects>, id: String) -> Result<Snapshot, String> {
    blocking(&state, move |p| p.delete(&id)).await
}

#[tauri::command]
pub async fn projects_copy(
    state: State<'_, Projects>,
    id: String,
    name: String,
) -> Result<Snapshot, String> {
    blocking(&state, move |p| p.copy(&id, name)).await
}

#[tauri::command]
pub async fn projects_import(
    state: State<'_, Projects>,
    window: tauri::WebviewWindow,
) -> Result<Snapshot, String> {
    let selected = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .add_filter("桌面项目包", &["lyproj"])
        .pick_file()
        .await;
    match selected {
        Some(path) => blocking(&state, move |p| p.import(path.path())).await,
        None => state.snapshot().map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub async fn projects_export(
    state: State<'_, Projects>,
    window: tauri::WebviewWindow,
    id: String,
) -> Result<Option<String>, String> {
    let path = rfd::AsyncFileDialog::new()
        .set_parent(&window)
        .add_filter("桌面项目包", &["lyproj"])
        .set_file_name(format!("project-{id}.lyproj"))
        .save_file()
        .await;
    let Some(path) = path else { return Ok(None) };
    blocking(&state, move |p| {
        p.export(&id, path.path())?;
        Ok(Some(path.path().display().to_string()))
    })
    .await
}
