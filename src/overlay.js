const { listen } = window.__TAURI__.event;

const label = document.getElementById("label");

listen("dictation-state", ({ payload }) => {
  document.body.dataset.state = payload;
  if (payload === "recording") label.textContent = "Listening…";
  else if (payload === "transcribing") label.textContent = "Transcribing…";
});
