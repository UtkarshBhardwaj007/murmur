# Murmur — Progress

## Milestones

- [x] 1. **Scaffold** — Tauri 2 project builds and launches a tray icon + empty settings window on the current OS. `cargo check` clean. Initialize git, first commit, create `PROGRESS.md`.
- [x] 2. **Audio capture** — record from default mic to a WAV on hotkey press (hardcoded hotkey for now); verify by inspecting the WAV's duration and sample rate in a test.
- [ ] 3. **STT integration** — model download manager with progress + checksum verification; sherpa-onnx transcribes the recorded WAV; add the mock-engine trait and tests.
- [ ] 4. **Hotkey + modes** — configurable global shortcut, push-to-talk and toggle modes both working; recording indicator overlay with state changes.
- [ ] 5. **Paste pipeline** — clipboard write, synthetic paste, clipboard restore; macOS Accessibility detection and guidance flow; transcript post-processing.
- [ ] 6. **Settings UI** — full settings window wired to the JSON store, including hotkey rebinding UI, model picker (with download-on-switch), launch-at-login.
- [ ] 7. **Polish & docs** — app icon, README with install instructions per OS (including unsigned-binary caveats and Wayland notes), CHANGELOG, LICENSE.
- [ ] 8. **CI green** — `ci.yml` passing on all three OS runners. Push and verify with `gh run watch`; fix cross-platform compile errors.
- [ ] 9. **Release** — bump to `v0.1.0`, tag, push, verify `release.yml` produces a GitHub Release with all five installer artifacts attached. Download one artifact via `gh release download` to confirm.

## Current status / next action

Milestone 2 done: cpal capture on a dedicated audio thread (Stream is !Send), mono
downmix + moving-average low-pass + linear resample to 16 kHz, WAV dump to
`<app-data>/recordings/last-recording.wav` on stop. Hardcoded Cmd/Ctrl+Shift+Space
toggle hotkey via tauri-plugin-global-shortcut, plus a tray-menu "Start/Stop Dictation"
item (doubles as the future Wayland fallback). tauri-plugin-log wired up.
Verified: 9 unit tests pass (resampler length/frequency/identity, WAV rate/duration/
clamping); clippy -D warnings clean; app launches and registers the hotkey (see log).
Real-mic capture can't be verified headlessly — added to manual-testing scope.

**Next:** Milestone 3 — STT integration: model download manager (progress + SHA-256),
sherpa-onnx with Parakeet TDT 0.6B v2 int8, whisper.cpp base.en alternative, SttEngine
trait + mock for tests.
