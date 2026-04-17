import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

const els = {
  cpsInput: document.getElementById("cpsInput"),
  cpsValue: document.getElementById("cpsValue"),
  cpsDelay: document.getElementById("cpsDelay"),
  toggleMeta: document.getElementById("toggleMeta"),
  inventoryMeta: document.getElementById("inventoryMeta"),
  statusBadge: document.getElementById("statusBadge"),
  toggleKey: document.getElementById("toggleKey"),
  inventoryKey: document.getElementById("inventoryKey"),
  bindToggleBtn: document.getElementById("bindToggleBtn"),
  bindInventoryBtn: document.getElementById("bindInventoryBtn"),
  bindHint: document.getElementById("bindHint"),
  noticeText: document.getElementById("noticeText"),
  privilegeBadge: document.getElementById("privilegeBadge"),
  startBtn: document.getElementById("startBtn"),
  stopBtn: document.getElementById("stopBtn"),
};

let pendingBindTarget = null;

function setPendingButtonState(button, isPending) {
  button.classList.toggle("binding", isPending);
  button.textContent = isPending ? "Press any key..." : "Change";
}

function refreshView(state) {
  const pendingToggle = state.pending_bind === "toggle";
  const pendingInventory = state.pending_bind === "inventory";
  pendingBindTarget = state.pending_bind ?? null;
  const delayMs = Math.max(1, Math.round(1000 / Math.max(1, Number(state.cps))));

  els.cpsInput.value = state.cps;
  els.cpsValue.textContent = state.cps;
  els.toggleKey.textContent = state.toggle_key;
  els.inventoryKey.textContent = state.inventory_key;
  els.cpsDelay.textContent = `${delayMs} ms`;
  els.toggleMeta.textContent = state.toggle_key;
  els.inventoryMeta.textContent = state.inventory_key;
  if (els.noticeText) {
    els.noticeText.textContent = state.notice;
  }

  els.statusBadge.classList.remove("running", "paused", "stopped");
  if (state.running) {
    els.statusBadge.classList.add("running");
  } else if (state.inv_paused) {
    els.statusBadge.classList.add("paused");
  } else {
    els.statusBadge.classList.add("stopped");
  }

  els.statusBadge.textContent = `State: ${state.status}`;
  els.privilegeBadge.textContent = state.is_elevated ? "Admin: yes" : "Admin: no";
  els.privilegeBadge.classList.toggle("elevated", state.is_elevated);
  els.privilegeBadge.classList.toggle("not-elevated", !state.is_elevated);

  setPendingButtonState(els.bindToggleBtn, pendingToggle);
  setPendingButtonState(els.bindInventoryBtn, pendingInventory);
  els.bindToggleBtn.disabled = pendingInventory;
  els.bindInventoryBtn.disabled = pendingToggle;

  if (pendingToggle) {
    els.bindHint.textContent = "Press any key to set Toggle. Escape cancels.";
  } else if (pendingInventory) {
    els.bindHint.textContent = "Press any key to set Inventory Pause. Escape cancels.";
  } else {
    els.bindHint.textContent = "Any keyboard key can be assigned. Duplicate binds are blocked.";
  }

  els.startBtn.disabled = state.running;
  els.stopBtn.disabled = !state.running && !state.inv_paused;
}

async function loadState() {
  const state = await invoke("get_state");
  refreshView(state);
}

function getBrowserKeyCode(event) {
  const keyCode = Number(event.keyCode || event.which || 0);
  return Number.isFinite(keyCode) ? keyCode : 0;
}

window.addEventListener("keydown", async (event) => {
  if (!pendingBindTarget || event.repeat) {
    return;
  }

  const keyCode = getBrowserKeyCode(event);
  if (!keyCode) {
    return;
  }

  event.preventDefault();
  event.stopPropagation();

  try {
    const state = await invoke("set_key_bind", {
      target: pendingBindTarget,
      keyCode
    });
    refreshView(state);
  } catch (error) {
    if (els.noticeText) {
      els.noticeText.textContent = `Error: ${String(error)}`;
    }
    pendingBindTarget = null;
  }
}, true);

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
  try {
    const state = await invoke("begin_key_bind", { target: "toggle" });
    refreshView(state);
  } catch (error) {
    if (els.noticeText) {
      els.noticeText.textContent = `Error: ${String(error)}`;
    }
  }
});

els.bindInventoryBtn.addEventListener("click", async () => {
  try {
    const state = await invoke("begin_key_bind", { target: "inventory" });
    refreshView(state);
  } catch (error) {
    if (els.noticeText) {
      els.noticeText.textContent = `Error: ${String(error)}`;
    }
  }
});

listen("state-updated", (event) => {
  refreshView(event.payload);
});

loadState().catch((error) => {
  if (els.noticeText) {
    els.noticeText.textContent = `Error: ${String(error)}`;
  }
  els.bindHint.textContent = "State loading failed. Please restart the app.";
});
