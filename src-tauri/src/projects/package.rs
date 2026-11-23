use super::{Project, check_real, invalid, new_project};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const FORMAT: &str = "ly-desktop-project";
const MAX_FILES: usize = 16_384;
const MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MANIFEST_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    version: u32,
    name: String,
    description: String,
    exported_at: u64,
    files: BTreeMap<String, Entry>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    bytes: u64,
    sha256: String,
}

fn valid_part(s: &str) -> bool {
    let stem = s
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_uppercase();
    let reserved = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|p| {
        stem.strip_prefix(p).is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    !s.is_empty()
        && !s.ends_with([' ', '.'])
        && !reserved
        && !s.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*'])
        && !s.chars().any(char::is_control)
}

fn check_path(s: &str) -> io::Result<()> {
    if !s.starts_with("portable/")
        || s.len() > 1024
        || s.split('/').count() > 32
        || !s.split('/').all(valid_part)
    {
        return Err(invalid(format!("项目资源路径无效：{s}")));
    }
    Ok(())
}

fn files_in(root: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    let mut found = Vec::new();
    let mut stack = vec![("portable".to_owned(), root.join("portable"))];
    while let Some((prefix, dir)) = stack.pop() {
        if !check_real(&dir)?.is_dir() {
            return Err(invalid("项目资源目录不可用"));
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| invalid("项目文件名编码无效"))?;
            let relative = format!("{prefix}/{name}");
            check_path(&relative)?;
            let path = entry.path();
            let meta = check_real(&path)?;
            if meta.is_dir() {
                stack.push((relative, path));
            } else if meta.is_file() {
                found.push((relative, path));
                if found.len() > MAX_FILES {
                    return Err(invalid("项目文件过多"));
                }
            } else {
                return Err(invalid("项目资源须为目录或普通文件"));
            }
        }
    }
    found.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    Ok(found)
}

fn copy_bytes(reader: &mut impl Read, writer: &mut impl Write, limit: u64) -> io::Result<Entry> {
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limit {
            return Err(invalid("项目内容超出允许大小"));
        }
        writer.write_all(&buf[..n])?;
        hash.update(&buf[..n]);
    }
    use std::fmt::Write as _;
    let mut sha256 = String::with_capacity(64);
    for byte in hash.finalize() {
        write!(sha256, "{byte:02x}").expect("format into String");
    }
    Ok(Entry {
        bytes: total,
        sha256,
    })
}

fn create_resource(root: &Path, relative: &str) -> io::Result<File> {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().ok_or_else(|| invalid("项目资源缺少父目录"))?)?;
    File::create_new(path)
}

pub(super) fn copy_portable(root: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir(dest.join("portable"))?;
    let mut remaining = MAX_BYTES;
    for (relative, path) in files_in(root)? {
        let mut target = create_resource(dest, &relative)?;
        remaining -= copy_bytes(&mut File::open(path)?, &mut target, remaining)?.bytes;
    }
    Ok(())
}

pub(super) fn pack(root: &Path, project: Project, output: &mut File) -> io::Result<()> {
    let mut writer = ZipWriter::new(output);
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .large_file(true);
    let mut remaining = MAX_BYTES;
    let mut entries = BTreeMap::new();
    for (relative, path) in files_in(root)? {
        writer.start_file(&relative, opts)?;
        let entry = copy_bytes(&mut File::open(path)?, &mut writer, remaining)?;
        remaining -= entry.bytes;
        entries.insert(relative, entry);
    }
    writer.start_file("manifest.json", opts)?;
    let manifest = Manifest {
        format: FORMAT.to_owned(),
        version: 1,
        name: project.name,
        description: project.description,
        exported_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_secs(),
        files: entries,
    };
    let manifest = serde_json::to_vec(&manifest)?;
    if manifest.len() as u64 > MANIFEST_LIMIT {
        return Err(invalid("项目包清单过大"));
    }
    writer.write_all(&manifest)?;
    writer.finish()?;
    Ok(())
}

pub(super) fn unpack(path: &Path, destination: &Path) -> io::Result<Project> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    if archive.has_duplicate_names() {
        return Err(invalid("项目包包含重复文件"));
    }
    if archive.len() > MAX_FILES + 1 {
        return Err(invalid("项目包文件过多"));
    }
    let manifest: Manifest = {
        let mut raw = Vec::new();
        archive
            .by_name("manifest.json")?
            .take(MANIFEST_LIMIT + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 > MANIFEST_LIMIT {
            return Err(invalid("项目包清单过大"));
        }
        serde_json::from_slice(&raw)?
    };
    if manifest.version != 1 || manifest.format != FORMAT {
        return Err(invalid("不支持的项目包格式或版本"));
    }
    if archive.len() != manifest.files.len() + 1 {
        return Err(invalid("项目包内容与清单不一致"));
    }
    let project = new_project(manifest.name, manifest.description)?;
    let mut remaining = MAX_BYTES;
    let mut names = HashSet::new();
    fs::create_dir(destination.join("portable"))?;
    for (name, entry) in manifest.files {
        check_path(&name)?;
        if !names.insert(name.to_uppercase()) {
            return Err(invalid("项目包文件名在 Windows 上冲突"));
        }
        let mut file = archive.by_name(&name)?;
        if !file.is_file()
            || file
                .unix_mode()
                .is_some_and(|m| !matches!(m & 0o170000, 0 | 0o100000))
            || file.size() != entry.bytes
            || entry.bytes > remaining
        {
            return Err(invalid(format!("文件类型或大小不符合清单：{name}")));
        }
        let actual = copy_bytes(
            &mut file,
            &mut create_resource(destination, &name)?,
            entry.bytes,
        )?;
        if actual.bytes != entry.bytes || actual.sha256 != entry.sha256 {
            return Err(invalid(format!("项目文件校验失败：{name}")));
        }
        remaining -= actual.bytes;
    }
    Ok(project)
}
