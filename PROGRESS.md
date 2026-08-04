# Murmur — Progress

## Milestones

- [x] 1. **Scaffold** — Tauri 2 project builds and launches a tray icon + empty settings window on the current OS. `cargo check` clean. Initialize git, first commit, create `PROGRESS.md`.
- [x] 2. **Audio capture** — record from default mic to a WAV on hotkey press (hardcoded hotkey for now); verify by inspecting the WAV's duration and sample rate in a test.
- [x] 3. **STT integration** — model download manager with progress + checksum verification; sherpa-onnx transcribes the recorded WAV; add the mock-engine trait and tests.
- [x] 4. **Hotkey + modes** — configurable global shortcut, push-to-talk and toggle modes both working; recording indicator overlay with state changes.
- [x] 5. **Paste pipeline** — clipboard write, synthetic paste, clipboard restore; macOS Accessibility detection and guidance flow; transcript post-processing.
- [x] 6. **Settings UI** — full settings window wired to the JSON store, including hotkey rebinding UI, model picker (with download-on-switch), launch-at-login.
- [x] 7. **Polish & docs** — app icon, README with install instructions per OS (including unsigned-binary caveats and Wayland notes), CHANGELOG, LICENSE.
- [x] 8. **CI green** — `ci.yml` passing on all three OS runners. Push and verify with `gh run watch`; fix cross-platform compile errors.
- [ ] 9. **Release** — bump to `v0.1.0`, tag, push, verify `release.yml` produces a GitHub Release with all five installer artifacts attached. Download one artifact via `gh release download` to confirm.

## Current status / next action

Milestone 8 done: CI green on ubuntu/macos/windows (run 30957637066). Fixes needed:
(1) pin windows-core 0.61 to unify cpal's windows crate graph; (2) Linux builds
sherpa-onnx from vendored source — the prebuilt "static" tarball ships no .a files;
(3) Windows: crt-static + trailing advapi32 link-arg + CMAKE_MSVC_RUNTIME_LIBRARY=
MultiThreaded (CMP0091=NEW) so whisper.cpp, sherpa, onnxruntime, and Rust all agree
on the static CRT; (4) dropped mobile-only staticlib/cdylib crate types.

**Next:** Milestone 9 — release.yml building bundles on all three runners for a v* tag,
artifact names murmur-<version>-<os>-<arch>.<ext>, GitHub Release with auto notes +
CHANGELOG; tag v0.1.0; verify all five installers attach; download one to confirm.
