use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    schema: u32,
    pub theme: Theme,
    pub mcp_enabled: bool,
    pub mcp_port: u16,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: 1,
            theme: Theme::System,
            mcp_enabled: false,
            mcp_port: 8765,
        }
    }
}

pub struct Preferences {
    path: PathBuf,
    current: Mutex<Settings>,
}
impl Preferences {
    pub fn open(root: &Path) -> Result<Self, String> {
        let path = root.join("settings.json");
        let current = match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(s) if s.schema == 1 && s.mcp_port != 0 => s,
                Ok(s) if s.schema > 1 => {
                    return Err(format!("设置版本 {} 高于本应用支持的版本", s.schema));
                }
                _ => {
                    let backup =
                        root.join(format!("settings.broken-{}.json", uuid::Uuid::new_v4()));
                    fs::rename(&path, &backup).map_err(|e| e.to_string())?;
                    tracing::warn!(file=%backup.display(),"设置文件已备份，已恢复默认值");
                    Settings::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => return Err(e.to_string()),
        };
        Ok(Self {
            path,
            current: Mutex::new(current),
        })
    }

    pub fn get(&self) -> Settings {
        self.current
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
        let mut guard = self.current.lock().map_err(|_| "设置锁不可用")?;
        let mut next = guard.clone();
        change(&mut next);
        let tmp = self.path.with_extension("tmp");
        let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(tmp, &self.path).map_err(|e| e.to_string())?;
        *guard = next.clone();
        Ok(next)
    }
}

#[tauri::command]
pub fn get_settings(state: tauri::State<Preferences>) -> Settings {
    state.get()
}
#[tauri::command]
pub fn set_theme(theme: Theme, state: tauri::State<Preferences>) -> Result<Settings, String> {
    state.update(|s| s.theme = theme)
}
