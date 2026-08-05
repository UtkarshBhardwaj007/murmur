//! Regression tests for the macOS bundle configuration.
//!
//! Murmur is signed with the hardened runtime. If the microphone entitlement
//! is dropped — or the config stops pointing at the entitlements file — macOS
//! hard-denies mic access with no prompt and no System Settings entry, which
//! bricks dictation for every macOS user. These tests pin the wiring.

use std::path::Path;

fn manifest_path(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn entitlements_grant_microphone_access() {
    let plist = std::fs::read_to_string(manifest_path("entitlements.plist"))
        .expect("entitlements.plist must exist next to tauri.conf.json");
    let key_pos = plist
        .find("com.apple.security.device.audio-input")
        .expect("audio-input entitlement missing from entitlements.plist");
    let after_key = &plist[key_pos..];
    assert!(
        after_key.contains("<true/>"),
        "audio-input entitlement must be set to true"
    );
}

#[test]
fn tauri_config_wires_up_signing_and_entitlements() {
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(manifest_path("tauri.conf.json")).expect("read tauri.conf.json"),
    )
    .expect("tauri.conf.json must be valid JSON");
    let macos = &config["bundle"]["macOS"];

    assert_eq!(
        macos["entitlements"], "entitlements.plist",
        "bundle.macOS.entitlements must reference the entitlements file"
    );
    assert_eq!(
        macos["signingIdentity"], "-",
        "bundle.macOS.signingIdentity must ad-hoc sign; an unsigned bundle \
         cannot hold TCC permission grants"
    );
}

#[test]
fn info_plist_explains_microphone_usage() {
    let plist = std::fs::read_to_string(manifest_path("Info.plist")).expect("read Info.plist");
    assert!(
        plist.contains("NSMicrophoneUsageDescription"),
        "microphone usage description is required for the permission prompt"
    );
}
