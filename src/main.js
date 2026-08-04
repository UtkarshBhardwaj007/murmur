const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const modelList = document.getElementById("model-list");

function formatBytes(bytes) {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.round(bytes / 1e3)} kB`;
}

function render(models) {
  modelList.replaceChildren(
    ...models.map((m) => {
      const row = document.createElement("div");
      row.className = "model-row";
      row.dataset.id = m.id;

      const info = document.createElement("div");
      info.className = "model-info";
      const name = document.createElement("div");
      name.className = "model-name";
      name.textContent = m.name + (m.active ? " · active" : "");
      const status = document.createElement("div");
      status.className = "model-status";
      status.textContent = m.installed
        ? "Installed"
        : `Not downloaded (${formatBytes(m.total_bytes)})`;
      info.append(name, status);

      const action = document.createElement("div");
      action.className = "model-action";
      if (!m.installed) {
        const btn = document.createElement("button");
        btn.textContent = "Download";
        btn.addEventListener("click", () => startDownload(m.id, row, btn));
        action.append(btn);
      }

      row.append(info, action);
      return row;
    })
  );
}

async function startDownload(id, row, btn) {
  btn.disabled = true;
  btn.textContent = "Downloading…";
  let bar = row.querySelector("progress");
  if (!bar) {
    bar = document.createElement("progress");
    bar.max = 1;
    bar.value = 0;
    row.querySelector(".model-info").append(bar);
  }
  try {
    await invoke("download_model", { id });
  } catch (e) {
    const status = row.querySelector(".model-status");
    status.textContent = `Download failed: ${e}`;
    status.classList.add("error");
    btn.disabled = false;
    btn.textContent = "Retry";
    return;
  }
  await refresh();
}

async function refresh() {
  render(await invoke("model_status"));
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

listen("model-download-complete", refresh);
listen("model-required", () => refresh());

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

// The permission can be granted while the window is open; re-check on focus.
window.addEventListener("focus", checkAccessibility);

checkAccessibility();
refresh();
