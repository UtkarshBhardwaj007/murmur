const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// The microphone/accessibility permission cards only exist on macOS; other
// platforms never need the status polling behind them.
const IS_MACOS = navigator.userAgent.includes("Macintosh");

// ---------------------------------------------------------------------------
// Settings state

let settings = null;
let models = [];

const hotkeyBtn = document.getElementById("hotkey-btn");
const hotkeyError = document.getElementById("hotkey-error");
const autoPaste = document.getElementById("auto-paste");
const launchAtLogin = document.getElementById("launch-at-login");
const modelList = document.getElementById("model-list");
const saveStatus = document.getElementById("save-status");

async function save(patch) {
  const next = { ...settings, ...patch };
  try {
    await invoke("set_settings", { new: next });
    settings = next;
    flashSaved();
    return true;
  } catch (e) {
    console.error("set_settings failed:", e);
    return e;
  }
}

let savedTimer;
function flashSaved() {
  saveStatus.hidden = false;
  clearTimeout(savedTimer);
  savedTimer = setTimeout(() => (saveStatus.hidden = true), 1200);
}

function renderSettings() {
  hotkeyBtn.textContent = settings.hotkey;
  document.querySelector(
    `input[name="mode"][value="${settings.mode}"]`
  ).checked = true;
  autoPaste.checked = settings.auto_paste;
  launchAtLogin.checked = settings.launch_at_login;
}

// ---------------------------------------------------------------------------
// Hotkey rebinding: click, then press the new combination.

let capturing = false;

hotkeyBtn.addEventListener("click", () => {
  capturing = true;
  hotkeyError.hidden = true;
  hotkeyBtn.textContent = "Press keys… (Esc to cancel)";
  hotkeyBtn.classList.add("capturing");
});

window.addEventListener(
  "keydown",
  async (e) => {
    if (!capturing) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.key === "Escape") {
      stopCapture();
      return;
    }
    // Wait for a non-modifier key to complete the combo.
    if (["Shift", "Control", "Alt", "Meta"].includes(e.key)) return;

    const mods = [];
    if (e.metaKey) mods.push("Cmd");
    if (e.ctrlKey) mods.push("Ctrl");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    if (mods.length === 0) {
      hotkeyError.textContent =
        "Use at least one modifier (⌘/Ctrl/Alt/Shift) so ordinary typing keeps working.";
      hotkeyError.hidden = false;
      return;
    }

    const combo = [...mods, e.code].join("+");
    stopCapture();
    const result = await save({ hotkey: combo });
    if (result !== true) {
      hotkeyError.textContent = String(result);
      hotkeyError.hidden = false;
    }
    renderSettings();
  },
  true
);

function stopCapture() {
  capturing = false;
  hotkeyBtn.classList.remove("capturing");
  renderSettings();
}

// ---------------------------------------------------------------------------
// Mode / behavior controls

for (const radio of document.querySelectorAll('input[name="mode"]')) {
  radio.addEventListener("change", async () => {
    if (radio.checked) {
      await save({ mode: radio.value });
      renderSettings();
    }
  });
}

autoPaste.addEventListener("change", async () => {
  await save({ auto_paste: autoPaste.checked });
  renderSettings();
});

launchAtLogin.addEventListener("change", async () => {
  const result = await save({ launch_at_login: launchAtLogin.checked });
  if (result !== true) launchAtLogin.checked = !launchAtLogin.checked;
  renderSettings();
});

// ---------------------------------------------------------------------------
// Models: radio picker with download-on-switch

function formatBytes(bytes) {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.round(bytes / 1e3)} kB`;
}

function renderModels() {
  modelList.replaceChildren(
    ...models.map((m) => {
      const row = document.createElement("label");
      row.className = "model-row";
      row.dataset.id = m.id;

      const radio = document.createElement("input");
      radio.type = "radio";
      radio.name = "model";
      radio.value = m.id;
      radio.checked = m.id === settings.model;
      radio.addEventListener("change", () => selectModel(m, row, radio));

      const info = document.createElement("div");
      info.className = "model-info";
      const name = document.createElement("div");
      name.className = "model-name";
      name.textContent = m.name;
      const status = document.createElement("div");
      status.className = "model-status";
      status.textContent = m.installed
        ? "Installed"
        : `Not downloaded (${formatBytes(m.total_bytes)})`;
      info.append(name, status);

      row.append(radio, info);
      return row;
    })
  );
}

// Model whose download this window is waiting on; when the
// model-download-complete event for it arrives, it becomes the active model.
let pendingModel = null;

async function selectModel(model, row, radio) {
  if (!model.installed) {
    let bar = row.querySelector("progress");
    if (!bar) {
      bar = document.createElement("progress");
      bar.max = 1;
      bar.value = 0;
      row.querySelector(".model-info").append(bar);
    }
    radio.disabled = true;
    pendingModel = model.id;
    try {
      // Returns as soon as the download starts; completion arrives via the
      // model-download-complete / model-download-error events.
      await invoke("download_model", { id: model.id });
    } catch (e) {
      pendingModel = null;
      showDownloadError(row, radio, e);
    }
    return;
  }
  await save({ model: model.id });
  await refreshModels();
}

function showDownloadError(row, radio, error) {
  const status = row.querySelector(".model-status");
  status.textContent = `Download failed: ${error}`;
  status.classList.add("error");
  radio.disabled = false;
  radio.checked = false;
  renderSettings();
  document.querySelector(
    `input[name="model"][value="${settings.model}"]`
  ).checked = true;
}

async function refreshModels() {
  models = await invoke("model_status");
  renderModels();
}

listen("model-download-progress", ({ payload }) => {
  const row = modelList.querySelector(`[data-id="${payload.model}"]`);
  if (!row) return;
  const bar = row.querySelector("progress");
  if (bar) bar.value = payload.downloaded / payload.total;
  const status = row.querySelector(".model-status");
  status.textContent = `Downloading ${payload.file} — ${formatBytes(
    payload.downloaded
  )} of ${formatBytes(payload.total)}`;
});

listen("model-download-complete", async ({ payload }) => {
  if (payload === pendingModel) {
    pendingModel = null;
    await save({ model: payload });
  }
  await refreshModels();
});

listen("model-download-error", ({ payload }) => {
  if (pendingModel === null) return;
  const row = modelList.querySelector(`[data-id="${pendingModel}"]`);
  pendingModel = null;
  if (!row) return;
  showDownloadError(row, row.querySelector('input[type="radio"]'), payload);
});

listen("model-required", refreshModels);

// ---------------------------------------------------------------------------
// Microphone guidance (macOS)

async function checkMicrophone() {
  const card = document.getElementById("microphone");
  try {
    card.hidden = (await invoke("microphone_status")) !== "denied";
  } catch {
    card.hidden = true;
  }
}

document
  .getElementById("open-microphone")
  .addEventListener("click", () => invoke("open_microphone_settings"));

listen("mic-denied", checkMicrophone);

// ---------------------------------------------------------------------------
// Accessibility guidance (macOS)

async function checkAccessibility() {
  const card = document.getElementById("accessibility");
  try {
    card.hidden = await invoke("accessibility_status");
  } catch {
    card.hidden = true;
  }
}

document
  .getElementById("grant-accessibility")
  .addEventListener("click", async () => {
    await invoke("request_accessibility");
    await checkAccessibility();
  });

document
  .getElementById("open-accessibility")
  .addEventListener("click", () => invoke("open_accessibility_settings"));

if (IS_MACOS) {
  window.addEventListener("focus", () => {
    checkAccessibility();
    checkMicrophone();
  });

  // Permissions can change in System Settings while this window sits open;
  // poll so the warning cards clear (or appear) without a restart.
  setInterval(() => {
    if (!document.hidden) {
      checkAccessibility();
      checkMicrophone();
    }
  }, 3000);
}

// ---------------------------------------------------------------------------
// Init

(async function init() {
  settings = await invoke("get_settings");
  renderSettings();
  await refreshModels();
  if (IS_MACOS) {
    await checkAccessibility();
    await checkMicrophone();
  }
})();
