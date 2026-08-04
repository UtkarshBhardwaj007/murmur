# Murmur — Progress

## Milestones

- [x] 1. **Scaffold** — Tauri 2 project builds and launches a tray icon + empty settings window on the current OS. `cargo check` clean. Initialize git, first commit, create `PROGRESS.md`.
- [x] 2. **Audio capture** — record from default mic to a WAV on hotkey press (hardcoded hotkey for now); verify by inspecting the WAV's duration and sample rate in a test.
- [x] 3. **STT integration** — model download manager with progress + checksum verification; sherpa-onnx transcribes the recorded WAV; add the mock-engine trait and tests.
- [x] 4. **Hotkey + modes** — configurable global shortcut, push-to-talk and toggle modes both working; recording indicator overlay with state changes.
- [ ] 5. **Paste pipeline** — clipboard write, synthetic paste, clipboard restore; macOS Accessibility detection and guidance flow; transcript post-processing.
- [ ] 6. **Settings UI** — full settings window wired to the JSON store, including hotkey rebinding UI, model picker (with download-on-switch), launch-at-login.
- [ ] 7. **Polish & docs** — app icon, README with install instructions per OS (including unsigned-binary caveats and Wayland notes), CHANGELOG, LICENSE.
- [ ] 8. **CI green** — `ci.yml` passing on all three OS runners. Push and verify with `gh run watch`; fix cross-platform compile errors.
- [ ] 9. **Release** — bump to `v0.1.0`, tag, push, verify `release.yml` produces a GitHub Release with all five installer artifacts attached. Download one artifact via `gh release download` to confirm.

## Current status / next action

Milestone 4 done: settings persistence (JSON via `directories` config dir, atomic
save, corrupt-file fallback to defaults), hotkey read from settings and re-appliable
via `apply_hotkey` (unregister_all + register), push-to-talk (Pressed→start,
Released→stop) and toggle modes with key-repeat suppression, and a transparent
always-on-top click-through overlay pill (bottom-center of the cursor's monitor)
driven by `overlay::apply(UiState)` which also fans out tray tooltip + events.
macOSPrivateApi enabled for window transparency.
Verified: 21 unit tests (settings roundtrip/corrupt/partial/mode-strings added);
clippy clean; app launches, loads default settings, registers hotkey. PTT key-up,
overlay visuals, and monitor placement need human eyes → MANUAL_TESTING (milestone 7).
NOTE for CI: sherpa-rs static on Linux wants RUSTFLAGS="-C relocation-model=dynamic-no-pic".

**Next:** Milestone 5 — paste pipeline: arboard clipboard write, enigo synthetic paste,
clipboard restore ~1 s later, macOS Accessibility detection + guidance, transcript
post-processing (trim, capitalize, strip partial trailing tokens) with tests.
