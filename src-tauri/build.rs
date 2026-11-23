use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "--short=10", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=APP_COMMIT={}", commit.trim());
    println!(
        "cargo:rustc-env=APP_BUILD_SECONDS={}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    println!("cargo:rerun-if-changed=../.git/HEAD");
    if let Ok(output) = Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
    {
        if output.status.success() {
            println!(
                "cargo:rerun-if-changed=../.git/{}",
                String::from_utf8_lossy(&output.stdout).trim()
            );
        }
    }
    tauri_build::build()
}
