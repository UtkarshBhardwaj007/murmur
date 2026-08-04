# Murmur — Progress

## Milestones

- [x] 1. **Scaffold** — Tauri 2 project builds and launches a tray icon + empty settings window on the current OS. `cargo check` clean. Initialize git, first commit, create `PROGRESS.md`.
- [x] 2. **Audio capture** — record from default mic to a WAV on hotkey press (hardcoded hotkey for now); verify by inspecting the WAV's duration and sample rate in a test.
- [x] 3. **STT integration** — model download manager with progress + checksum verification; sherpa-onnx transcribes the recorded WAV; add the mock-engine trait and tests.
- [x] 4. **Hotkey + modes** — configurable global shortcut, push-to-talk and toggle modes both working; recording indicator overlay with state changes.
- [x] 5. **Paste pipeline** — clipboard write, synthetic paste, clipboard restore; macOS Accessibility detection and guidance flow; transcript post-processing.
- [ ] 6. **Settings UI** — full settings window wired to the JSON store, including hotkey rebinding UI, model picker (with download-on-switch), launch-at-login.
- [ ] 7. **Polish & docs** — app icon, README with install instructions per OS (including unsigned-binary caveats and Wayland notes), CHANGELOG, LICENSE.
- [ ] 8. **CI green** — `ci.yml` passing on all three OS runners. Push and verify with `gh run watch`; fix cross-platform compile errors.
- [ ] 9. **Release** — bump to `v0.1.0`, tag, push, verify `release.yml` produces a GitHub Release with all five installer artifacts attached. Download one artifact via `gh release download` to confirm.

## Current status / next action

Milestone 5 done: `postprocess::post_process` (noise markers like [BLANK_AUDIO]/
(inaudible), SentencePiece ▁, trailing partial tokens, whitespace collapse, first-
letter capitalization) with 9 tests. `paste::deliver`: clipboard backup (text or
image) → write transcript → enigo Cmd/Ctrl+V → 1 s → restore backup; on paste
failure the transcript stays on the clipboard. macOS Accessibility detected via
macos-accessibility-client; startup + settings-window guidance card with "Grant
permission" (system prompt) and "Open System Settings" buttons. Info.plist with
NSMicrophoneUsageDescription added for the bundle. auto_paste=false → clipboard only.
Verified: 30 unit tests; clippy clean; app launch log shows accessibility detection
firing (debug binary untrusted → guidance shown). Actual keystroke synthesis needs
a human → MANUAL_TESTING.
NOTE for CI: sherpa-rs static on Linux wants RUSTFLAGS="-C relocation-model=dynamic-no-pic".

**Next:** Milestone 6 — full settings UI wired to the JSON store: hotkey rebinding,
mode picker, model picker with download-on-switch, auto-paste toggle, launch-at-login.
