mod commands;
mod package;

pub use commands::*;

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    projects: Vec<Project>,
    active_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    version: u32,
    #[serde(flatten)]
    project: Project,
}

#[derive(Clone)]
pub struct Projects(Arc<Inner>);

struct Inner {
    root: PathBuf,
    state: Mutex<Snapshot>,
}

fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn unique_path(parent: &Path, prefix: &str) -> PathBuf {
    parent.join(format!("{prefix}{}", Uuid::new_v4()))
}

fn check_real(path: &Path) -> io::Result<fs::Metadata> {
    let m = path.symlink_metadata()?;
    #[cfg(windows)]
    if std::os::windows::fs::MetadataExt::file_attributes(&m) & 0x400 != 0 {
        return Err(invalid(format!("项目路径包含重解析点：{}", path.display())));
    }
    if m.is_symlink() {
        return Err(invalid(format!("项目路径包含链接：{}", path.display())));
    }
    Ok(m)
}

pub(super) fn atomic_file<T>(
    target: &Path,
    write: impl FnOnce(&mut File) -> io::Result<T>,
) -> io::Result<T> {
    let parent = target.parent().ok_or_else(|| invalid("缺少父目录"))?;
    let temp = unique_path(parent, ".write-");
    let mut file = File::create_new(&temp)?;
    let result = write(&mut file).and_then(|value| file.sync_all().map(|_| value));
    drop(file);
    let result = result.and_then(|value| fs::rename(&temp, target).map(|_| value));
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

struct Staging {
    path: PathBuf,
}

impl Staging {
    fn create(root: &Path) -> io::Result<Self> {
        let path = unique_path(root, ".new-");
        fs::create_dir(&path)?;
        Ok(Self { path })
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        // commit 后目录已改名；失败时只删除本次创建的暂存目录。
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn metadata(dir: &Path) -> io::Result<Project> {
    check_real(dir)?;
    let path = dir.join("project.json");
    if check_real(&path)?.len() > 64 * 1024 {
        return Err(invalid("项目清单过大"));
    }
    let document: Metadata = serde_json::from_reader(File::open(path)?)?;
    if document.version != 1
        || dir.file_name().and_then(|v| v.to_str()) != Some(document.project.id.as_str())
    {
        return Err(invalid("项目版本或身份与目录不符"));
    }
    if !check_real(&dir.join("portable"))?.is_dir() {
        return Err(invalid("项目资源目录不可用"));
    }
    Ok(document.project)
}

fn write_metadata(dir: &Path, project: &Project) -> io::Result<()> {
    atomic_file(&dir.join("project.json"), |file| {
        Ok(serde_json::to_writer_pretty(
            file,
            &Metadata {
                version: 1,
                project: project.clone(),
            },
        )?)
    })
}

fn new_project(name: String, description: String) -> io::Result<Project> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || description.chars().count() > 2000 {
        return Err(invalid(
            "请输入 1 至 120 字符的项目名称，描述最长 2000 字符",
        ));
    }
    Ok(Project {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        description: description.trim().to_owned(),
    })
}

impl Projects {
    pub fn open(app_root: &Path) -> Result<Self, String> {
        Self::load(app_root).map_err(|err| format!("打开项目资料失败：{err}"))
    }

    fn load(app_root: &Path) -> io::Result<Self> {
        let root = app_root.join("projects");
        fs::create_dir_all(&root)?;
        check_real(&root)?;
        let mut projects = Vec::new();
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(id) = name.to_str() else { continue };
            if Uuid::parse_str(id).is_err() {
                continue;
            }
            match metadata(&entry.path()) {
                Ok(project) => projects.push(project),
                Err(err) => tracing::warn!(project = id, %err, "跳过不可读取的项目目录"),
            }
        }
        projects.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
        let active_path = root.join("current.json");
        let saved: Option<String> = match fs::read(&active_path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(id) => id,
                Err(err) => {
                    let backup = unique_path(&root, ".damaged-current-");
                    fs::rename(&active_path, &backup)?;
                    tracing::warn!(%err, backup=?backup, "已保留损坏的当前项目记录");
                    None
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(err),
        };
        let active_id = saved.filter(|id| projects.iter().any(|p| p.id == *id));
        Ok(Self(Arc::new(Inner {
            root,
            state: Mutex::new(Snapshot {
                projects,
                active_id,
            }),
        })))
    }

    fn state(&self) -> io::Result<MutexGuard<'_, Snapshot>> {
        self.0.state.lock().map_err(|_| error("项目状态不可用"))
    }

    fn dir(&self, id: &str) -> io::Result<PathBuf> {
        if Uuid::parse_str(id).ok().map(|v| v.to_string()).as_deref() != Some(id) {
            return Err(invalid("无效的项目编号"));
        }
        Ok(self.0.root.join(id))
    }

    fn save_current(&self, id: Option<&str>) -> io::Result<()> {
        atomic_file(&self.0.root.join("current.json"), |f| {
            serde_json::to_writer(&mut *f, &id)?;
            f.write_all(b"\n")
        })
    }

    fn snapshot(&self) -> io::Result<Snapshot> {
        Ok(self.state()?.clone())
    }

    fn install(&self, staged: Staging, project: Project) -> io::Result<Snapshot> {
        write_metadata(&staged.path, &project)?;
        let target = self.dir(&project.id)?;
        let mut state = self.state()?;
        fs::rename(&staged.path, &target)?;
        if state.projects.is_empty() {
            if let Err(e) = self.save_current(Some(&project.id)) {
                if let Err(rollback) = fs::rename(&target, &staged.path) {
                    return Err(error(format!(
                        "保存当前项目失败：{e}。回滚失败：{rollback}"
                    )));
                }
                return Err(e);
            }
            state.active_id = Some(project.id.clone());
        }
        state.projects.push(project);
        state
            .projects
            .sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
        Ok(state.clone())
    }

    fn create(&self, name: String, description: String) -> io::Result<Snapshot> {
        let project = new_project(name, description)?;
        let staged = Staging::create(&self.0.root)?;
        fs::create_dir(staged.path.join("portable"))?;
        self.install(staged, project)
    }

    fn source(&self, id: &str) -> io::Result<(Project, PathBuf)> {
        let dir = self.dir(id)?;
        if !self.state()?.projects.iter().any(|p| p.id == id) {
            return Err(error("找不到项目"));
        }
        Ok((metadata(&dir)?, dir))
    }

    fn switch(&self, id: &str) -> io::Result<Snapshot> {
        let mut state = self.state()?;
        if !state.projects.iter().any(|p| p.id == id) {
            return Err(error("找不到项目"));
        }
        metadata(&self.dir(id)?)?;
        self.save_current(Some(id))?;
        state.active_id = Some(id.to_owned());
        Ok(state.clone())
    }

    fn rename(&self, id: &str, name: String, description: String) -> io::Result<Snapshot> {
        let mut project = new_project(name, description)?;
        let mut state = self.state()?;
        let entry = state
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| error("找不到项目"))?;
        let dir = self.dir(id)?;
        metadata(&dir)?;
        project.id = id.to_owned();
        write_metadata(&dir, &project)?;
        *entry = project;
        state
            .projects
            .sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
        Ok(state.clone())
    }

    fn delete(&self, id: &str) -> io::Result<Snapshot> {
        let mut state = self.state()?;
        let idx = state
            .projects
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| error("找不到项目"))?;
        let path = self.dir(id)?;
        check_real(&path)?;
        let trash = unique_path(&self.0.root, ".deleted-");
        fs::rename(&path, &trash)?;
        if state.active_id.as_deref() == Some(id) {
            if let Err(e) = self.save_current(None) {
                fs::rename(&trash, &path)?;
                return Err(e);
            }
            state.active_id = None;
        }
        state.projects.remove(idx);
        let snapshot = state.clone();
        drop(state);
        if let Err(err) = fs::remove_dir_all(&trash) {
            tracing::warn!(path=?trash,%err,"项目已从列表移除，资料目录仍待清理");
        }
        Ok(snapshot)
    }

    fn copy(&self, id: &str, name: String) -> io::Result<Snapshot> {
        let (source, path) = self.source(id)?;
        let project = new_project(name, source.description)?;
        let staged = Staging::create(&self.0.root)?;
        package::copy_portable(&path, &staged.path)?;
        self.install(staged, project)
    }

    fn import(&self, path: &Path) -> io::Result<Snapshot> {
        let staged = Staging::create(&self.0.root)?;
        let project = package::unpack(path, &staged.path)?;
        self.install(staged, project)
    }

    fn export(&self, id: &str, target: &Path) -> io::Result<()> {
        let (project, path) = self.source(id)?;
        let parent = target
            .parent()
            .ok_or_else(|| invalid("缺少目标路径"))?
            .canonicalize()?;
        let app_root = self
            .0
            .root
            .parent()
            .ok_or_else(|| invalid("缺少资料目录"))?
            .canonicalize()?;
        if parent.starts_with(app_root) {
            return Err(invalid("请将项目包保存在应用资料目录之外"));
        }
        atomic_file(target, |f| package::pack(&path, project, f))
    }
}

#[cfg(test)]
mod tests;
