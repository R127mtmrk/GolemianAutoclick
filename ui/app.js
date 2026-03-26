const { invoke } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;

const els = {
  cpsInput: document.getElementById("cpsInput"),
  cpsValue: document.getElementById("cpsValue"),
  statusBadge: document.getElementById("statusBadge"),
  toggleKey: document.getElementById("toggleKey"),
  inventoryKey: document.getElementById("inventoryKey"),
  bindToggleBtn: document.getElementById("bindToggleBtn"),
  bindInventoryBtn: document.getElementById("bindInventoryBtn"),
  bindHint: document.getElementById("bindHint"),
  startBtn: document.getElementById("startBtn"),
  stopBtn: document.getElementById("stopBtn")
};

function refreshView(state) {
  els.cpsInput.value = state.cps;
  els.cpsValue.textContent = state.cps;
  els.toggleKey.textContent = state.toggle_key;
  els.inventoryKey.textContent = state.inventory_key;

  els.statusBadge.classList.remove("running", "paused", "stopped");
  if (state.running) {
    els.statusBadge.classList.add("running");
  } else if (state.inv_paused) {
    els.statusBadge.classList.add("paused");
  } else {
    els.statusBadge.classList.add("stopped");
  }

  els.statusBadge.textContent = `State: ${state.status}`;

  if (state.pending_bind === "toggle") {
    els.bindHint.textContent = "Press any key to set Toggle.";
  } else if (state.pending_bind === "inventory") {
    els.bindHint.textContent = "Press any key to set Inventory Pause.";
  } else {
    els.bindHint.textContent = "You can assign any keyboard key.";
  }
}

async function loadState() {
  const state = await invoke("get_state");
  refreshView(state);
}

els.cpsInput.addEventListener("input", async () => {
  const cps = Number(els.cpsInput.value);
  els.cpsValue.textContent = String(cps);
  const state = await invoke("set_cps", { cps });
  refreshView(state);
});

els.startBtn.addEventListener("click", async () => {
  const state = await invoke("set_running", { running: true });
  refreshView(state);
});

els.stopBtn.addEventListener("click", async () => {
  const state = await invoke("set_running", { running: false });
  refreshView(state);
});

els.bindToggleBtn.addEventListener("click", async () => {
  const state = await invoke("begin_key_bind", { target: "toggle" });
  refreshView(state);
});

els.bindInventoryBtn.addEventListener("click", async () => {
  const state = await invoke("begin_key_bind", { target: "inventory" });
  refreshView(state);
});

listen("state-updated", (event) => {
  refreshView(event.payload);
});

loadState().catch((error) => {
  els.bindHint.textContent = `Error: ${String(error)}`;
});
