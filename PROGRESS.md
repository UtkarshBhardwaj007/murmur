# Murmur — Progress

## Milestones

- [x] 1. **Scaffold** — Tauri 2 project builds and launches a tray icon + empty settings window on the current OS. `cargo check` clean. Initialize git, first commit, create `PROGRESS.md`.
- [x] 2. **Audio capture** — record from default mic to a WAV on hotkey press (hardcoded hotkey for now); verify by inspecting the WAV's duration and sample rate in a test.
- [x] 3. **STT integration** — model download manager with progress + checksum verification; sherpa-onnx transcribes the recorded WAV; add the mock-engine trait and tests.
- [x] 4. **Hotkey + modes** — configurable global shortcut, push-to-talk and toggle modes both working; recording indicator overlay with state changes.
- [x] 5. **Paste pipeline** — clipboard write, synthetic paste, clipboard restore; macOS Accessibility detection and guidance flow; transcript post-processing.
- [x] 6. **Settings UI** — full settings window wired to the JSON store, including hotkey rebinding UI, model picker (with download-on-switch), launch-at-login.
- [x] 7. **Polish & docs** — app icon, README with install instructions per OS (including unsigned-binary caveats and Wayland notes), CHANGELOG, LICENSE.
- [ ] 8. **CI green** — `ci.yml` passing on all three OS runners. Push and verify with `gh run watch`; fix cross-platform compile errors.
- [ ] 9. **Release** — bump to `v0.1.0`, tag, push, verify `release.yml` produces a GitHub Release with all five installer artifacts attached. Download one artifact via `gh release download` to confirm.

## Current status / next action

Milestone 7 done: app icon (purple gradient + white sound-wave, generated at 1024px,
run through `tauri icon` for all platform sizes), tray icons for idle/recording/
transcribing states swapped at runtime, README (features, per-OS install with
Gatekeeper/SmartScreen/Wayland notes, screenshot placeholders, privacy, build docs),
CHANGELOG (Keep a Changelog, 0.1.0 section), MIT LICENSE, docs/MANUAL_TESTING.md
with per-OS human checklists.
Verified: 30 tests, clippy clean, app launches with the new tray icon without errors.

**Next:** Milestone 8 — ci.yml (fmt, clippy -D warnings, test, debug build on the
three OS runners) + push to GitHub and iterate until green. Remember Linux needs
webkit2gtk/alsa apt packages and sherpa static needs RUSTFLAGS=-C relocation-model=dynamic-no-pic.
