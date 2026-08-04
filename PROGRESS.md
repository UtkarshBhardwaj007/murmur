# Murmur — Progress

## Milestones

- [x] 1. **Scaffold** — Tauri 2 project builds and launches a tray icon + empty settings window on the current OS. `cargo check` clean. Initialize git, first commit, create `PROGRESS.md`.
- [x] 2. **Audio capture** — record from default mic to a WAV on hotkey press (hardcoded hotkey for now); verify by inspecting the WAV's duration and sample rate in a test.
- [x] 3. **STT integration** — model download manager with progress + checksum verification; sherpa-onnx transcribes the recorded WAV; add the mock-engine trait and tests.
- [ ] 4. **Hotkey + modes** — configurable global shortcut, push-to-talk and toggle modes both working; recording indicator overlay with state changes.
- [ ] 5. **Paste pipeline** — clipboard write, synthetic paste, clipboard restore; macOS Accessibility detection and guidance flow; transcript post-processing.
- [ ] 6. **Settings UI** — full settings window wired to the JSON store, including hotkey rebinding UI, model picker (with download-on-switch), launch-at-login.
- [ ] 7. **Polish & docs** — app icon, README with install instructions per OS (including unsigned-binary caveats and Wayland notes), CHANGELOG, LICENSE.
- [ ] 8. **CI green** — `ci.yml` passing on all three OS runners. Push and verify with `gh run watch`; fix cross-platform compile errors.
- [ ] 9. **Release** — bump to `v0.1.0`, tag, push, verify `release.yml` produces a GitHub Release with all five installer artifacts attached. Download one artifact via `gh release download` to confirm.

## Current status / next action

Milestone 3 done: model registry with pinned SHA-256 checksums (Parakeet TDT 0.6B v2
int8 via sherpa-onnx, whisper.cpp ggml base.en), streaming download manager with
progress events + hash-while-downloading verification + .part/rename atomicity.
`SttEngine` trait with sherpa (nemo_transducer), whisper-rs, and mock implementations;
transcription runs on a background thread and the engine stays loaded between
dictations. Minimal model-download UI in the settings window.
sherpa-rs uses `static` + `download-binaries` features (dynamic linking left the
binary without an rpath for libonnxruntime on macOS — static fixes launch AND
installer bundling). NOTE for CI (milestone 8): sherpa-rs static on Linux warns it
needs RUSTFLAGS="-C relocation-model=dynamic-no-pic".
Verified: 15 unit tests pass; both real engines transcribe a `say`-generated
"hello world this is a test" WAV correctly (ignored tests `--test real_engine`,
run locally with both models downloaded + checksum-verified end-to-end through
`examples/download_model.rs`); clippy clean; app launches statically linked.

**Next:** Milestone 4 — configurable global shortcut, push-to-talk + toggle modes,
recording indicator overlay window with state changes.
