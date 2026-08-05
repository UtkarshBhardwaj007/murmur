# Manual testing checklist

Things a human must verify by hand — they involve microphones, global
hotkeys, synthetic keystrokes, and OS permission prompts that automated
tests cannot exercise. Run through this list on each OS before a release.

Logs are the first debugging stop:

- macOS: `~/Library/Logs/com.murmur.murmur/murmur.log`
- Linux: `~/.local/share/com.murmur.murmur/logs/murmur.log` (XDG data dir)
- Windows: `%LOCALAPPDATA%\com.murmur.murmur\logs\murmur.log`

## Every OS

### First run
- [ ] App starts with a tray icon (purple wave); no window flash other than
      the settings window.
- [ ] Settings window opens; the model list shows Parakeet (~661 MB, active)
      and Whisper base.en (~148 MB), neither installed.
- [ ] Clicking a model radio starts the download; progress bar and byte
      counts update; killing the network mid-download shows an error and a
      retry works (partial `.part` files are re-downloaded, never trusted).
- [ ] After download, dictating for the first time loads the model (a few
      seconds; tray tooltip says "transcribing…").

### Dictation — toggle mode (default)
- [ ] Press the hotkey in a text editor: the "Listening…" pill appears
      bottom-center of the monitor with the cursor; tray icon gains a red dot.
- [ ] Speak a sentence, press the hotkey again: pill switches to
      "Transcribing…", then the text is pasted at the cursor with the first
      letter capitalized.
- [ ] Clipboard contents from before dictation (copy some text first) are
      restored ~1 s after the paste.
- [ ] Copy an **image** (screenshot), dictate, paste-restore returns the image.
- [ ] Dictating silence pastes nothing and returns to idle.

### Dictation — push-to-talk
- [ ] Switch mode in Settings. Hold the hotkey: recording only while held;
      release stops and transcribes.
- [ ] Holding the key down does not machine-gun start/stop (key repeat is
      suppressed).

### Settings
- [ ] Rebind the hotkey (e.g. `Ctrl+Alt+D`): old binding stops working, new
      one works immediately and survives an app restart.
- [ ] A hotkey without a modifier is rejected with an explanation.
- [ ] Esc cancels hotkey capture.
- [ ] Turn auto-paste off: transcript lands on the clipboard only.
- [ ] Launch at login: toggle on, log out/in (or check
      System Settings → Login Items / `~/.config/autostart` / registry Run
      key), Murmur is running.
- [ ] Switch active model to Whisper: download runs, then transcription uses
      it (log line `loaded WhisperBaseEn`).
- [ ] Close the settings window: app stays in the tray; tray → Settings…
      reopens it.
- [ ] Tray → Start/Stop Dictation behaves exactly like the hotkey (toggle).
- [ ] Tray → Quit exits the process.

## macOS specifics
- [ ] First recording triggers **exactly one** system Microphone permission
      prompt (with the explanatory text). After clicking Allow, no further
      prompts ever appear — including after quitting and relaunching the
      app. (Regression check for the linker-signed-bundle bug: multiple
      stacked prompts per recording that return on every launch.)
- [ ] `codesign -dvv /Applications/Murmur.app` shows
      `Identifier=com.murmur.app` and does **not** show `linker-signed`.
- [ ] Denying the mic prompt shows the "Microphone access is blocked" card
      in Settings; the button opens the Privacy & Security → Microphone
      pane; after enabling Murmur there, dictation works without restart.
- [ ] Without **Accessibility** permission, the settings window shows the
      yellow "Enable auto-paste" card; "Grant permission…" triggers the
      system prompt; after enabling Murmur in System Settings and
      restarting, the card disappears and auto-paste works.
- [ ] Gatekeeper: fresh download opens after `xattr -cr` or right-click →
      Open (README instructions are accurate).
- [ ] The `.dmg` installs a universal binary (`lipo -archs Murmur` shows
      `x86_64 arm64`).
- [ ] Overlay pill does not steal focus from the app being dictated into.

## Windows specifics
- [ ] SmartScreen "More info → Run anyway" flow works for both `.msi` and
      NSIS `.exe` (README instructions are accurate).
- [ ] Paste works in Notepad, a browser text box, and a terminal
      (Ctrl+V-based; some terminals need Ctrl+Shift+V — known limitation,
      transcript remains on the clipboard).

## Linux specifics
- [ ] **X11:** global hotkey and auto-paste work in a GTK app, a Qt app, and
      a browser.
- [ ] **Wayland (GNOME and KDE):** app starts and shows the tray icon; if
      the hotkey cannot register, a log line says so; tray →
      Start/Stop Dictation still records and copies to clipboard; document
      any paste failure (expected on some compositors).
- [ ] `.AppImage` runs on a distro without dev packages installed.
- [ ] `.deb` installs and `murmur` appears in the application menu.
