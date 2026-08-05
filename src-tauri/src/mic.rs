//! Microphone permission gate.
//!
//! On macOS, touching any audio-input HAL object while the permission is
//! undetermined makes the system queue its own TCC prompt — and cpal touches
//! several per recording (device lookup, format query, AudioUnit creation,
//! start). Recording must therefore never reach cpal until access is
//! explicitly authorized; this module owns that decision.
//!
//! The decision logic ([`plan_start`]) is pure and unit-tested; only the
//! thin [`platform`] layer talks to AVFoundation.

use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MicPermission {
    /// The user granted access; recording may proceed.
    Authorized,
    /// The user has never been asked (or never answered).
    NotDetermined,
    /// The user denied access, or a policy restricts it.
    Denied,
}

/// What the dictation pipeline should do given the current permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPlan {
    /// Open the microphone and record.
    Record,
    /// Ask the OS to show its one permission prompt, then re-evaluate.
    RequestPermission,
    /// Don't touch the microphone; walk the user to System Settings.
    GuideToSettings,
}

/// Pure decision: how to react to a dictation-start request.
pub fn plan_start(permission: MicPermission) -> StartPlan {
    match permission {
        MicPermission::Authorized => StartPlan::Record,
        MicPermission::NotDetermined => StartPlan::RequestPermission,
        MicPermission::Denied => StartPlan::GuideToSettings,
    }
}

/// Resolve the permission as far as possible without user interaction
/// beyond the OS prompt: authorized passes through, undetermined triggers
/// the (single) system prompt and waits up to `timeout` for the answer,
/// denied stays denied.
///
/// Blocks the calling thread while the prompt is up — never call on the
/// main/UI thread.
pub fn ensure_authorized(timeout: Duration) -> MicPermission {
    match plan_start(platform::status()) {
        StartPlan::Record => MicPermission::Authorized,
        StartPlan::RequestPermission => platform::request_access(timeout),
        StartPlan::GuideToSettings => MicPermission::Denied,
    }
}

/// Current permission without prompting.
pub fn status() -> MicPermission {
    platform::status()
}

#[cfg(target_os = "macos")]
mod platform {
    use std::sync::{mpsc, Mutex};
    use std::time::Duration;

    use objc2::runtime::Bool;
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeAudio};

    use super::MicPermission;

    fn audio_media_type() -> &'static objc2_av_foundation::AVMediaType {
        // SAFETY: reading a constant exported by AVFoundation.
        unsafe { AVMediaTypeAudio }.expect("AVMediaTypeAudio is always present")
    }

    pub fn status() -> MicPermission {
        // SAFETY: documented to accept AVMediaTypeAudio.
        let status =
            unsafe { AVCaptureDevice::authorizationStatusForMediaType(audio_media_type()) };
        match status {
            AVAuthorizationStatus::Authorized => MicPermission::Authorized,
            AVAuthorizationStatus::NotDetermined => MicPermission::NotDetermined,
            // Denied or Restricted (or anything unknown): treat as denied.
            _ => MicPermission::Denied,
        }
    }

    pub fn request_access(timeout: Duration) -> MicPermission {
        let (tx, rx) = mpsc::channel::<bool>();
        // The completion handler runs once on an arbitrary queue; the Mutex
        // makes the captured Sender safe to take from whichever thread that is.
        let tx = Mutex::new(Some(tx));
        let handler = block2::RcBlock::new(move |granted: Bool| {
            if let Some(tx) = tx.lock().expect("completion tx lock").take() {
                let _ = tx.send(granted.as_bool());
            }
        });
        log::info!("requesting microphone permission from the user");
        // SAFETY: documented to accept AVMediaTypeAudio; the block is kept
        // alive by the system until it is invoked.
        unsafe {
            AVCaptureDevice::requestAccessForMediaType_completionHandler(
                audio_media_type(),
                &handler,
            );
        }
        match rx.recv_timeout(timeout) {
            Ok(true) => MicPermission::Authorized,
            Ok(false) => MicPermission::Denied,
            // The user left the prompt unanswered; don't record, don't nag.
            Err(_) => MicPermission::NotDetermined,
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use std::time::Duration;

    use super::MicPermission;

    /// Windows and Linux have no blocking microphone-consent dialog for
    /// desktop apps; capture either works or fails at the device level.
    pub fn status() -> MicPermission {
        MicPermission::Authorized
    }

    pub fn request_access(_timeout: Duration) -> MicPermission {
        MicPermission::Authorized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorized_records_without_prompting() {
        assert_eq!(plan_start(MicPermission::Authorized), StartPlan::Record);
    }

    #[test]
    fn undetermined_requests_exactly_one_prompt() {
        assert_eq!(
            plan_start(MicPermission::NotDetermined),
            StartPlan::RequestPermission
        );
    }

    #[test]
    fn denied_never_touches_the_microphone() {
        assert_eq!(
            plan_start(MicPermission::Denied),
            StartPlan::GuideToSettings
        );
    }

    #[test]
    fn permission_serializes_to_stable_strings() {
        // The settings UI matches on these exact strings.
        assert_eq!(
            serde_json::to_string(&MicPermission::Authorized).unwrap(),
            "\"authorized\""
        );
        assert_eq!(
            serde_json::to_string(&MicPermission::NotDetermined).unwrap(),
            "\"not_determined\""
        );
        assert_eq!(
            serde_json::to_string(&MicPermission::Denied).unwrap(),
            "\"denied\""
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_is_always_authorized() {
        assert_eq!(status(), MicPermission::Authorized);
        assert_eq!(
            ensure_authorized(std::time::Duration::from_millis(1)),
            MicPermission::Authorized
        );
    }
}
