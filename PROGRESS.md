# Murmur — Progress

## Milestones

- [x] 1. **Scaffold** — Tauri 2 project builds and launches a tray icon + empty settings window on the current OS. `cargo check` clean. Initialize git, first commit, create `PROGRESS.md`.
- [x] 2. **Audio capture** — record from default mic to a WAV on hotkey press (hardcoded hotkey for now); verify by inspecting the WAV's duration and sample rate in a test.
- [x] 3. **STT integration** — model download manager with progress + checksum verification; sherpa-onnx transcribes the recorded WAV; add the mock-engine trait and tests.
- [x] 4. **Hotkey + modes** — configurable global shortcut, push-to-talk and toggle modes both working; recording indicator overlay with state changes.
- [x] 5. **Paste pipeline** — clipboard write, synthetic paste, clipboard restore; macOS Accessibility detection and guidance flow; transcript post-processing.
- [x] 6. **Settings UI** — full settings window wired to the JSON store, including hotkey rebinding UI, model picker (with download-on-switch), launch-at-login.
- [ ] 7. **Polish & docs** — app icon, README with install instructions per OS (including unsigned-binary caveats and Wayland notes), CHANGELOG, LICENSE.
- [ ] 8. **CI green** — `ci.yml` passing on all three OS runners. Push and verify with `gh run watch`; fix cross-platform compile errors.
- [ ] 9. **Release** — bump to `v0.1.0`, tag, push, verify `release.yml` produces a GitHub Release with all five installer artifacts attached. Download one artifact via `gh release download` to confirm.

## Current status / next action

Milestone 6 done: get_settings/set_settings commands (hotkey re-registration with
revert-on-failure, launch-at-login via tauri-plugin-autostart, model switch), full
settings window: click-to-capture hotkey rebinding (KeyboardEvent.code → plugin
format, requires a modifier, Esc cancels), mode radio (toggle/PTT), model radio
picker with download-on-switch + progress bar + revert on failure, auto-paste and
launch-at-login checkboxes, auto-save with "Saved" flash.
Verified: 30 unit tests, clippy clean, app launches with no errors in log. UI
interactions (rebinding, download-on-switch, autostart entry) need a human →
MANUAL_TESTING.
NOTE for CI: sherpa-rs static on Linux wants RUSTFLAGS="-C relocation-model=dynamic-no-pic".

**Next:** Milestone 7 — polish & docs: app icon, README (per-OS install incl.
unsigned-binary caveats + Wayland notes), CHANGELOG, LICENSE, docs/MANUAL_TESTING.md,
tray icon state variants.
