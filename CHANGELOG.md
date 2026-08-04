# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/bhardwajutkarsh/murmur/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bhardwajutkarsh/murmur/releases/tag/v0.1.0
