#![cfg(windows)]
use super::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, atomic::AtomicI64},
};

fn config() -> (Config, SigningKey) {
    let mut bytes = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(bytes.as_mut()).unwrap();
    let key = SigningKey::from_bytes(&bytes);
    (
        Config {
            server_url: "https://license.example.invalid".into(),
            product_id: "ca7e80ee-e477-4b35-9e5b-f195e9f3bf04".into(),
            trusted_keys: BTreeMap::from([(
                "test-key".into(),
                STANDARD.encode(key.verifying_key().as_bytes()),
            )]),
        },
        key,
    )
}
fn signed_bundle(
    key: &SigningKey,
    manager: &LicenseManager,
    now: i64,
    expires: i64,
    revision: i64,
) -> OfflineBundle {
    let identity = DeviceIdentity::from_seed(manager.identity.seed);
    let license_id = "ac378614-1475-456d-85c9-d6cf979d2363";
    let serial = uuid::Uuid::new_v4().to_string();
    let claims = json!({"ver":2,"mode":"offline_device","lid":license_id,"kid":"LIC-test","product_id":manager.config.product_id,"prd":"desktop-test","serial":serial,"revision":revision,"binding_version":1,"device_key_hash":identity.key_hash(),"fingerprint_hash":manager.identity.fingerprint.hash,"fingerprint_version":1,"iat":now-60,"nbf":now-60,"exp":expires,"ent":{},"seat":1});
    let header = json!({"alg":"EdDSA","crv":"Ed25519","typ":"GMT","kid":"test-key"});
    let message = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap()),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
    );
    let token = format!(
        "{message}.{}",
        URL_SAFE_NO_PAD.encode(key.sign(message.as_bytes()).to_bytes())
    );
    OfflineBundle {
        token,
        state: grantmint_offline::VerificationState {
            product_id: manager.config.product_id.clone(),
            license_id: license_id.into(),
            device_key_hash: identity.key_hash(),
            revision,
            binding_version: 1,
            max_seen_time: now,
            serial,
        },
    }
}
fn status_at(manager: &mut LicenseManager, time: i64) -> LicenseStatus {
    manager.set_test_time(time);
    manager.status()
}
fn open_clock(cfg: Config, path: PathBuf) -> LicenseManager {
    LicenseManager::open_with_clock(cfg, path, Arc::new(AtomicI64::new(utc_now()))).unwrap()
}
fn install(manager: &mut LicenseManager, bundle: OfflineBundle) {
    manager.install_test_fixture(bundle).unwrap();
}

#[test]
fn expiry_and_clock_rollback_after_restart_preserve_authorization_state() {
    let dir = tempfile::tempdir().unwrap();
    let (cfg, key) = config();
    let now = utc_now();
    let mut manager = open_clock(cfg.clone(), dir.path().to_owned());
    let bundle = signed_bundle(&key, &manager, now, now + 8 * 86400, 1);
    install(&mut manager, bundle);
    assert_eq!(status_at(&mut manager, now + 86400 - 1).state, "valid");
    assert_eq!(status_at(&mut manager, now + 86400).state, "expiring");
    assert_eq!(
        status_at(&mut manager, now + 8 * 86400 - 1).state,
        "expiring"
    );
    assert_eq!(status_at(&mut manager, now + 8 * 86400).state, "expired");
    drop(manager);
    let mut manager = open_clock(cfg, dir.path().to_owned());
    assert_eq!(status_at(&mut manager, now).state, "invalid");
    assert_eq!(status_at(&mut manager, now + 8 * 86400).state, "expired");
}

#[test]
fn restoring_only_an_old_certificate_cannot_lower_persisted_revision() {
    let dir = tempfile::tempdir().unwrap();
    let (cfg, key) = config();
    let now = utc_now();
    let mut manager = open_clock(cfg.clone(), dir.path().to_owned());
    let first = signed_bundle(&key, &manager, now, now + 86400, 1);
    install(&mut manager, first);
    let previous = fs::read(dir.path().join("state.bin")).unwrap();
    let renewed = signed_bundle(&key, &manager, now, now + 86400 * 30, 2);
    install(&mut manager, renewed);
    drop(manager);
    fs::write(dir.path().join("state.bin"), previous).unwrap();
    let mut manager = open_clock(cfg, dir.path().to_owned());
    assert_eq!(manager.status().state, "invalid");
}
