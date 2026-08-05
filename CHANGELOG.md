# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-08-05

### Fixed

- macOS asked for microphone permission repeatedly (several stacked prompts
  per recording, on every launch). The app bundle was only linker-signed, so
  macOS could not persist the permission grant; bundles are now properly
  ad-hoc signed (verified in CI) and recording waits for explicit
  microphone authorization before opening the device, so the system prompt
  appears exactly once.
- Microphone access was hard-denied on macOS — no prompt and no System
  Settings entry — because the hardened-runtime signature carried no
  entitlements. The bundle now ships the `audio-input` entitlement, and the
  release workflow fails if it ever goes missing.
- Denied microphone access now shows a guided "open System Settings" card
  instead of failing silently.
- The permission cards in Settings now update live (no restart), and the
  Accessibility card explains how to refresh a grant that went stale after
  an app update (remove Murmur from the list, then re-add it).

### Changed

- Releases are now published automatically when a version-bump PR merges to
  `main`.

## [0.1.0] - 2026-08-04

### Added

- Global-hotkey dictation with **toggle** and **push-to-talk** modes
  (default `Cmd+Shift+Space` on macOS, `Ctrl+Shift+Space` elsewhere).
- Fully on-device speech-to-text: **NVIDIA Parakeet TDT 0.6B v2** (int8,
  sherpa-onnx) by default, **whisper.cpp base.en** as a smaller alternative.
- First-run model download with progress bar and pinned SHA-256 checksum
  verification; fully offline afterwards.
- Auto-paste at the cursor via synthetic Cmd/Ctrl+V, with the previous
  clipboard contents (text or image) restored ~1 second later; optional
  clipboard-only mode.
- Transcript post-processing: noise-marker removal, whitespace cleanup,
  first-letter capitalization, trailing partial-token stripping.
- System tray icon with idle/recording/transcribing states and a
  **Start/Stop Dictation** menu item (also the Wayland fallback).
- Floating always-on-top "Listening…/Transcribing…" indicator pill.
- Settings window: hotkey rebinding, mode picker, model picker with
  download-on-switch, auto-paste toggle, launch at login.
- macOS Accessibility permission detection with a guided grant flow.
- Installers: `.dmg` (universal macOS), `.msi` + NSIS `.exe` (Windows),
  `.AppImage` + `.deb` (Linux).

[Unreleased]: https://github.com/UtkarshBhardwaj007/murmur/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/UtkarshBhardwaj007/murmur/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/UtkarshBhardwaj007/murmur/releases/tag/v0.1.0
