# Murmur

**Local, private dictation for your desktop.** Press a shortcut, speak, and the
transcript appears at your cursor — transcribed entirely on your device.
No account, no cloud, no audio ever leaving your computer.

![Murmur demo](docs/assets/demo.gif)
<!-- TODO: record demo GIF: press hotkey → speak → text appears in editor -->

## Features

- **One shortcut, anywhere.** Default `Cmd+Shift+Space` (macOS) /
  `Ctrl+Shift+Space` (Windows/Linux), rebindable in Settings.
- **Two modes.** *Toggle* (press to start, press to stop) or *push-to-talk*
  (record while held).
- **100 % on-device speech-to-text.** NVIDIA Parakeet TDT 0.6B v2 (int8,
  via [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)) by default, or
  [whisper.cpp](https://github.com/ggerganov/whisper.cpp) `base.en` if you
  prefer a ~150 MB download over a ~660 MB one. Models are fetched once from
  their official sources, verified against pinned SHA-256 checksums, and
  everything works offline afterwards.
- **Pastes where you're typing.** The transcript lands on your clipboard and
  is pasted at the cursor; your previous clipboard contents are restored a
  second later. Auto-paste can be turned off.
- **Stays out of the way.** A tray icon and a small floating pill while
  recording — that's it.

| Settings | Recording indicator |
| --- | --- |
| ![Settings window](docs/assets/settings.png) | ![Recording pill](docs/assets/overlay.png) |
<!-- TODO: add real screenshots -->

## Install

Grab the installer for your OS from the
[latest release](../../releases/latest).

### macOS (`.dmg`)

Murmur is not code-signed (no Apple Developer account), so Gatekeeper will
complain the first time:

1. Open the `.dmg` and drag **Murmur** to Applications.
2. Either right-click **Murmur.app** → **Open** → **Open**, or clear the
   quarantine flag in a terminal:

   ```sh
   xattr -cr /Applications/Murmur.app
   ```

3. On first dictation, grant **Microphone** access when prompted.
4. For auto-paste, grant **Accessibility** access: System Settings →
   Privacy & Security → Accessibility → enable Murmur (the app walks you
   through this).

### Windows (`.msi` or `-setup.exe`)

The installers are unsigned, so SmartScreen will show *"Windows protected
your PC"*: click **More info** → **Run anyway**. Then just follow the
installer.

### Linux (`.AppImage` or `.deb`)

```sh
# AppImage
chmod +x murmur-*.AppImage && ./murmur-*.AppImage

# Debian/Ubuntu
sudo apt install ./murmur-*.deb
```

**X11 is fully supported** (global hotkey + auto-paste). On **Wayland**:

- Global shortcuts generally cannot be registered by ordinary apps. Use the
  tray menu's **Start/Stop Dictation** as a fallback, or configure a custom
  compositor shortcut that triggers it.
- Synthetic paste depends on the compositor; if it doesn't work, the
  transcript is still on the clipboard — paste manually.

## First run

On first launch Murmur opens Settings and offers to download the speech
model (with a progress bar). After that, dictation works fully offline:

1. Press the shortcut — a small pill appears: **Listening…**
2. Speak.
3. Press the shortcut again (toggle mode) or release it (push-to-talk).
4. The transcript is pasted at your cursor.

## Settings

Settings live in a JSON file in your platform's config directory
(`~/Library/Application Support/com.murmur.murmur/settings.json` on macOS,
`~/.config/murmur/settings.json` on Linux, `%APPDATA%\murmur\config\settings.json`
on Windows):

| Setting | Values | Default |
| --- | --- | --- |
| `hotkey` | any modifier + key combo | `Cmd/Ctrl+Shift+Space` |
| `mode` | `toggle`, `push_to_talk` | `toggle` |
| `model` | `parakeet-tdt-0.6b-v2-int8`, `whisper-base-en` | Parakeet |
| `auto_paste` | `true` / `false` (false = clipboard only) | `true` |
| `launch_at_login` | `true` / `false` | `false` |

## Privacy

Audio is recorded only while dictation is active, processed in memory on
your machine, and never sent anywhere. The only network requests Murmur
makes are the one-time model downloads from Hugging Face. The last
recording is kept at `<app-data>/recordings/last-recording.wav` for
debugging; delete it whenever you like.

## Building from source

Prerequisites: [Rust](https://rustup.rs), Node.js ≥ 18, and
[Tauri's platform dependencies](https://tauri.app/start/prerequisites/)
(on Linux: `libwebkit2gtk-4.1-dev`, `libasound2-dev`, and friends; cmake is
required for whisper.cpp).

```sh
npm install
npm run tauri dev     # run in development
npm run tauri build   # produce installers in src-tauri/target/release/bundle/
```

Run the tests:

```sh
cd src-tauri && cargo test
```

The STT engines sit behind a trait, so the test suite runs with a mock and
never downloads models. Opt-in end-to-end tests live in
`src-tauri/tests/real_engine.rs` (see the file header for how to run them).

## License

[MIT](LICENSE)
