mod fingerprint;
mod storage;

use base64::{Engine, engine::general_purpose::STANDARD};
use grantmint_offline::{DeviceIdentity, ExpectedDevice, OfflineBundle, TrustedKeys};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

use fingerprint::Fingerprint;
use storage::Store;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub server_url: String,
    pub product_id: String,
    pub trusted_keys: BTreeMap<String, String>,
}

impl Config {
    fn keys(&self) -> Result<TrustedKeys, String> {
        if uuid::Uuid::parse_str(&self.product_id)
            .map(|id| id.to_string())
            .ok()
            .as_deref()
            != Some(&self.product_id)
        {
            return Err("应用授权标识未配置".into());
        }
        let url = reqwest::Url::parse(&self.server_url).map_err(|_| "授权服务地址未配置")?;
        let local = cfg!(debug_assertions)
            && url.scheme() == "http"
            && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
        if (!local && url.scheme() != "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("授权服务须使用不含凭据的 HTTPS 地址".into());
        }
        if self.trusted_keys.is_empty() {
            return Err("授权公钥未配置".into());
        }
        self.trusted_keys
            .iter()
            .map(|(id, value)| {
                let bytes = STANDARD.decode(value).map_err(|_| "授权公钥编码无效")?;
                let key = bytes.try_into().map_err(|_| "授权公钥长度无效")?;
                Ok((id.clone(), key))
            })
            .collect()
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Identity {
    product_id: String,
    seed: [u8; 32],
    fingerprint: Fingerprint,
}
impl Drop for Identity {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.seed.zeroize();
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Anchor {
    max_time: i64,
    accepted: Option<grantmint_offline::VerificationState>,
}
#[derive(Clone, Serialize, Deserialize, Default)]
struct SavedState {
    bundle: Option<OfflineBundle>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    pub state: String,

    pub message: String,
    pub expires_at: Option<String>,
    pub warning_at: Option<String>,
    pub license_id: Option<String>,
    pub configured: bool,
}
impl LicenseStatus {
    fn new(state: &str, message: String, configured: bool) -> Self {
        Self {
            state: state.into(),
            message,
            expires_at: None,
            warning_at: None,
            license_id: None,
            configured,
        }
    }
}

pub struct LicenseManager {
    config: Config,
    keys: TrustedKeys,
    store: Store,
    identity: Identity,
    saved: SavedState,
    anchor: Anchor,
    base_time: i64,
    base_instant: Instant,
    recovery: Option<String>,
    #[cfg(test)]
    test_clock: Option<std::sync::Arc<std::sync::atomic::AtomicI64>>,
}

impl LicenseManager {
    pub fn open(config: Config, root: std::path::PathBuf) -> Result<Self, String> {
        let keys = config.keys()?;
        let now = utc_now();
        let store = Store::open_or_initialize(root, |store| {
            let mut seed = [0; 32];
            getrandom::getrandom(&mut seed).map_err(|_| "系统随机数不可用")?;
            let identity = Identity {
                product_id: config.product_id.clone(),
                seed,
                fingerprint: Fingerprint::collect(&config.product_id, None)?,
            };
            store.write("identity.bin", &identity)?;
            let anchor = Anchor {
                max_time: now,
                accepted: None,
            };
            store.write("anchor.bin", &anchor)?;
            store.write("state.bin", &SavedState::default())
        })?;
        let identity: Identity = store.read("identity.bin")?;
        if identity.product_id != config.product_id {
            return Err("应用标识与已保存的设备身份不一致".into());
        }
        let anchor: Anchor = store.read("anchor.bin")?;
        let (saved, recovery) = match store.read("state.bin") {
            Ok(saved) => (saved, None),
            Err(error) => (SavedState::default(), Some(error)),
        };
        let base_time = anchor.max_time;
        Ok(Self {
            config,
            keys,
            store,
            identity,
            saved,
            anchor,
            base_time,
            base_instant: Instant::now(),
            recovery,
            #[cfg(test)]
            test_clock: None,
        })
    }

    #[cfg(test)]
    pub fn open_with_clock(
        config: Config,
        root: std::path::PathBuf,
        clock: std::sync::Arc<std::sync::atomic::AtomicI64>,
    ) -> Result<Self, String> {
        let mut manager = Self::open(config, root)?;
        manager.test_clock = Some(clock);
        Ok(manager)
    }
    #[cfg(test)]
    pub fn set_test_time(&self, time: i64) {
        self.test_clock
            .as_ref()
            .expect("manual clock fixture required")
            .store(time, std::sync::atomic::Ordering::SeqCst);
    }

    fn wall_now(&self) -> i64 {
        #[cfg(test)]
        if let Some(clock) = self.test_clock.as_ref() {
            return clock.load(std::sync::atomic::Ordering::SeqCst);
        }
        utc_now()
    }

    fn clock(&self, wall: i64) -> Result<i64, String> {
        let floor = self
            .base_time
            .saturating_add(self.base_instant.elapsed().as_secs() as i64)
            .max(self.anchor.max_time);
        if wall.saturating_add(300) < floor {
            return Err("系统时钟回拨，请校正系统时间并重新检查授权".into());
        }
        Ok(wall.max(floor))
    }

    fn persist_anchor(&mut self, next: Anchor) -> Result<(), String> {
        self.store.write("anchor.bin", &next)?;
        if next.max_time
            > self
                .base_time
                .saturating_add(self.base_instant.elapsed().as_secs() as i64)
        {
            self.base_time = next.max_time;
            self.base_instant = Instant::now();
        }
        self.anchor = next;
        Ok(())
    }

    fn expected<'a>(&'a self, license_id: &'a str) -> ExpectedDevice<'a> {
        ExpectedDevice {
            product_id: &self.config.product_id,
            license_id,
            fingerprint_hash: &self.identity.fingerprint.hash,
            fingerprint_version: 1,
        }
    }

    fn check_storage_identity(&self) -> Result<(), String> {
        let identity: Identity = self.store.read("identity.bin")?;
        if identity.product_id != self.identity.product_id
            || identity.seed != self.identity.seed
            || identity.fingerprint.hash != self.identity.fingerprint.hash
            || identity.fingerprint.with_firmware != self.identity.fingerprint.with_firmware
        {
            return Err("受保护的设备身份被替换".into());
        }
        let anchor: Anchor = self.store.read("anchor.bin")?;
        if serde_json::to_vec(&anchor).ok() != serde_json::to_vec(&self.anchor).ok() {
            return Err("受保护的授权时间或版本记录被替换".into());
        }
        Ok(())
    }
    fn check_storage_certificate(&self) -> Result<(), String> {
        let saved: SavedState = self.store.read("state.bin")?;
        if serde_json::to_vec(&saved).ok() != serde_json::to_vec(&self.saved).ok() {
            return Err("授权证书文件发生外部变化，请重启后核验".into());
        }
        Ok(())
    }

    fn check_highwater(&self, state: &grantmint_offline::VerificationState) -> Result<(), String> {
        if let Some(high) = self.anchor.accepted.as_ref() {
            if state.license_id != high.license_id
                || state.product_id != high.product_id
                || state.device_key_hash != high.device_key_hash
                || state.revision < high.revision
                || state.binding_version != high.binding_version
                || (state.revision == high.revision && state.serial != high.serial)
            {
                return Err("授权状态低于已接受版本，拒绝旧证书回滚".into());
            }
        }
        Ok(())
    }

    fn check_fingerprint(&self) -> Result<(), String> {
        let current = Fingerprint::collect(
            &self.config.product_id,
            Some(self.identity.fingerprint.with_firmware),
        )?;
        if current.hash != self.identity.fingerprint.hash {
            return Err("设备指纹与激活设备不一致".into());
        }
        Ok(())
    }

    pub fn status(&mut self) -> LicenseStatus {
        self.status_at(self.wall_now())
    }

    fn status_at(&mut self, wall: i64) -> LicenseStatus {
        let mut result = LicenseStatus::new(
            "unactivated",
            "尚未激活，请输入授权编号和激活秘密".into(),
            true,
        );
        if let Err(error) = self.check_storage_identity() {
            result.state = "invalid".into();
            result.message = error;
            return result;
        }
        if let Err(error) = self.check_storage_certificate() {
            self.recovery = Some(error);
        }
        if self.saved.bundle.is_none() && self.anchor.accepted.is_some() {
            self.recovery = Some("已激活的本机缺少已接受的离线证书".into());
        }
        if let Some(error) = self.recovery.as_ref() {
            return LicenseStatus::new("recovery", format!("{error}；请联系管理员恢复授权"), true);
        }
        let now = match self.clock(wall) {
            Ok(now) => now,
            Err(error) => {
                result.state = "invalid".into();
                result.message = error;
                return result;
            }
        };
        if let Some(bundle) = self.saved.bundle.as_ref() {
            result.license_id = Some(bundle.state.license_id.clone());
            let mut state = bundle.state.clone();
            state.max_seen_time = state.max_seen_time.max(now);
            match self
                .check_highwater(&state)
                .and_then(|_| self.check_fingerprint())
                .and_then(|_| {
                    grantmint_offline::verify(
                        &bundle.token,
                        &self.keys,
                        &self.expected(&state.license_id),
                        &DeviceIdentity::from_seed(self.identity.seed),
                        &state,
                        now,
                        0,
                    )
                    .map_err(|error| format!("授权验证失败：{error}"))
                }) {
                Ok(verified) => {
                    let crossed_expiry =
                        self.anchor.max_time < verified.claims.exp && now >= verified.claims.exp;
                    let changed_certificate = self.anchor.accepted.as_ref().is_none_or(|old| {
                        old.serial != verified.state.serial
                            || old.revision != verified.state.revision
                    });
                    if changed_certificate
                        || crossed_expiry
                        || now.saturating_sub(self.anchor.max_time) >= 60
                    {
                        let anchor = Anchor {
                            max_time: verified.state.max_seen_time,
                            accepted: Some(verified.state),
                        };
                        if let Err(error) = self.persist_anchor(anchor) {
                            return LicenseStatus::new("invalid", error, true);
                        }
                    }
                    result.expires_at = timestamp(verified.claims.exp);
                    result.warning_at = timestamp(verified.claims.exp.saturating_sub(7 * 86400));
                    result.state = match verified.status {
                        grantmint_offline::Status::Valid => "valid",
                        grantmint_offline::Status::Expiring => "expiring",
                        grantmint_offline::Status::Expired => "expired",
                    }
                    .into();
                    result.message = match verified.status {
                        grantmint_offline::Status::Valid => "授权有效，可离线使用",
                        grantmint_offline::Status::Expiring => {
                            "授权将在七天内到期，请续约并联网更新授权"
                        }
                        grantmint_offline::Status::Expired => {
                            "授权已到期，请联系管理员续期并更新授权"
                        }
                    }
                    .into();
                }
                Err(error) => {
                    result.state = "invalid".into();
                    result.message = error;
                }
            }
        }
        result
    }
}

fn timestamp(value: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(value, 0).map(|time| time.to_rfc3339())
}
fn utc_now() -> i64 {
    chrono::Utc::now().timestamp()
}

static MANAGER: OnceLock<Mutex<Result<LicenseManager, String>>> = OnceLock::new();
const PRODUCT_CONFIG: &str = include_str!("../../licensing.json");

pub fn initialize(root: std::path::PathBuf) {
    MANAGER.get_or_init(|| {
        let result = serde_json::from_str::<Config>(PRODUCT_CONFIG)
            .map_err(|_| "授权配置格式无效".to_string())
            .and_then(|config| LicenseManager::open(config, root.join("licensing")));
        Mutex::new(result)
    });
}

fn with_manager<T>(
    operation: impl FnOnce(&mut LicenseManager) -> Result<T, String>,
) -> Result<T, String> {
    let manager = MANAGER.get().ok_or("授权服务未初始化")?;
    let mut guard = manager.lock().map_err(|_| "授权状态锁损坏")?;
    operation(guard.as_mut().map_err(|error| error.clone())?)
}
#[tauri::command]
pub async fn license_status() -> Result<LicenseStatus, String> {
    tauri::async_runtime::spawn_blocking(|| match with_manager(|manager| Ok(manager.status())) {
        Ok(status) => Ok(status),
        Err(message) => {
            let configured = serde_json::from_str::<Config>(PRODUCT_CONFIG)
                .is_ok_and(|config| config.keys().is_ok());
            Ok(LicenseStatus::new(
                if configured {
                    "invalid"
                } else {
                    "unconfigured"
                },
                message,
                configured,
            ))
        }
    })
    .await
    .map_err(|_| "授权状态任务失败".to_string())?
}
mod network;
#[cfg(test)]
mod tests;
#[tauri::command]
pub async fn license_activate(
    license_id: String,
    activation_secret: String,
) -> Result<LicenseStatus, String> {
    network::license_activate(license_id, activation_secret).await
}
#[tauri::command]
pub async fn license_sync() -> Result<LicenseStatus, String> {
    network::license_sync().await
}
