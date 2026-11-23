use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

pub struct Store {
    root: PathBuf,
    _lease: File,
}

impl Store {
    pub fn open_or_initialize(
        root: PathBuf,
        initialize: impl FnOnce(&Store) -> Result<(), String>,
    ) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|_| "不能创建授权安全存储目录")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|_| "无法限制授权目录权限")?;
        }
        ensure_directory(&root)?;
        let lease = secure_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("instance.lock"))
            .map_err(|_| "无法打开授权进程锁")?;
        lease
            .try_lock()
            .map_err(|_| "授权存储正在被另一个应用进程使用")?;
        let mut store = Self {
            root,
            _lease: lease,
        };
        let live = store.root.join("current");
        if present(&live)? {
            ensure_directory(&live)?;
            store.root = live;
            return Ok(store);
        }
        for file in ["identity.bin", "anchor.bin", "state.bin"] {
            if present(&store.root.join(file))? {
                return Ok(store);
            }
        }

        let staging = store.root.join(".initializing");
        if present(&staging)? {
            ensure_directory(&staging)?;
            fs::remove_dir_all(&staging).map_err(|_| "无法清理未提交的授权暂存目录")?;
        }
        fs::create_dir(&staging).map_err(|_| "无法创建授权暂存目录")?;
        store.root = staging;
        initialize(&store)?;
        publish(&store.root, &live)?;
        store.root = live;
        Ok(store)
    }

    pub fn read<T: DeserializeOwned>(&self, name: &str) -> Result<T, String> {
        let path = self.root.join(name);
        if fs::symlink_metadata(&path)
            .map_err(|_| "授权保护文件缺失")?
            .file_type()
            .is_symlink()
        {
            return Err("授权保护文件不能是链接".into());
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|_| "无法读取授权保护文件")?
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "无法读取授权保护文件")?;
        if bytes.len() > 1024 * 1024 {
            return Err("授权保护文件过大".into());
        }
        let plain = Zeroizing::new(unprotect(&bytes)?);
        serde_json::from_slice(&plain).map_err(|_| "授权保护文件内容损坏".into())
    }

    pub fn write<T: Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        let plain = Zeroizing::new(serde_json::to_vec(value).map_err(|_| "无法编码授权状态")?);
        let bytes = protect(&plain)?;
        let pending = self
            .root
            .join(format!(".{}-{}.pending", name, uuid::Uuid::new_v4()));
        let target = self.root.join(name);
        let result = (|| {
            let mut file = secure_options()
                .write(true)
                .create_new(true)
                .open(&pending)
                .map_err(|_| "无法写入授权保护文件")?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| "无法可靠保存授权保护文件")?;
            drop(file);
            replace(&pending, &target)?;
            #[cfg(unix)]
            File::open(&self.root)
                .and_then(|file| file.sync_all())
                .map_err(|_| "无法同步授权目录")?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(pending);
        }
        result
    }
}

fn present(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("无法检查授权存储".into()),
    }
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|_| "无法检查授权目录")?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err("授权目录不能是重解析点".into());
        }
    }
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("授权目录类型无效".into());
    }
    Ok(())
}

fn publish(source: &Path, target: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH;
        move_file(source, target, MOVEFILE_WRITE_THROUGH)
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, target).map_err(|_| "无法提交授权初始化")?;
        File::open(target.parent().ok_or("授权目录缺少父目录")?)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| "无法同步授权目录".into())
    }
}

fn secure_options() -> OpenOptions {
    let options = OpenOptions::new();
    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = options;
        options.mode(0o600);
        options
    };
    options
}

#[cfg(windows)]
fn replace(source: &Path, target: &Path) -> Result<(), String> {
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    move_file(
        source,
        target,
        MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
    )
}

#[cfg(windows)]
fn move_file(source: &Path, target: &Path, flags: u32) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), flags) } == 0 {
        Err("无法原子替换授权保护文件".into())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace(source: &Path, target: &Path) -> Result<(), String> {
    fs::rename(source, target).map_err(|_| "无法原子替换授权保护文件".into())
}

#[cfg(windows)]
fn crypt(bytes: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr().cast_mut(),
    };
    let entropy = b"ly-desktop.license.v2.current-user";
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: entropy.len() as u32,
        pbData: entropy.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err("DPAPI 无法读取或保护授权状态，请使用原 Windows 账号并联系维护人员".into());
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        if !encrypt {
            std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
        }
        LocalFree(output.pbData.cast());
    }
    Ok(result)
}
#[cfg(windows)]
fn protect(bytes: &[u8]) -> Result<Vec<u8>, String> {
    crypt(bytes, true)
}
#[cfg(windows)]
fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, String> {
    crypt(bytes, false)
}

#[cfg(not(windows))]
fn protect(_bytes: &[u8]) -> Result<Vec<u8>, String> {
    Err("当前发行版仅支持 Windows DPAPI 授权安全存储".into())
}
#[cfg(not(windows))]
fn unprotect(_bytes: &[u8]) -> Result<Vec<u8>, String> {
    Err("当前发行版仅支持 Windows DPAPI 授权安全存储".into())
}
