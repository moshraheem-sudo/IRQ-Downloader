const DEFAULTS = { port: 9624, intercept: true, minSizeKB: 0, bubbleSize: "normal", bubbleScale: null, smartPdf: true, pdfMode: "standard" };
const OLD_SIZES = { small: 85, normal: 100, large: 125 };

function saved() {
  const el = document.getElementById("saved");
  el.textContent = "Saved ✔";
  setTimeout(() => (el.textContent = ""), 1500);
}

async function init() {
  const opts = Object.assign({}, DEFAULTS, await chrome.storage.local.get(DEFAULTS));
  const intercept = document.getElementById("intercept");
  const minSize = document.getElementById("minSize");
  const port = document.getElementById("port");
  intercept.checked = opts.intercept;
  minSize.value = opts.minSizeKB;
  port.value = opts.port;
  const scale = document.getElementById("bubbleScale");
  const scaleText = document.getElementById("bubbleScaleText");
  const pct = opts.bubbleScale != null ? Math.round(opts.bubbleScale * 100) : OLD_SIZES[opts.bubbleSize] || 100;
  scale.value = String(pct);
  scaleText.textContent = `${pct}%`;
  scale.addEventListener("input", () => {
    scaleText.textContent = `${scale.value}%`;
    chrome.storage.local.set({ bubbleScale: Number(scale.value) / 100 });
  });
  scale.addEventListener("change", saved);
  intercept.addEventListener("change", () => chrome.storage.local.set({ intercept: intercept.checked }).then(saved));
  minSize.addEventListener("change", () =>
    chrome.storage.local.set({ minSizeKB: Math.max(0, Number(minSize.value) || 0) }).then(saved)
  );
  port.addEventListener("change", () => {
    const p = Number(port.value);
    if (p >= 1024 && p <= 65535) chrome.storage.local.set({ port: p }).then(saved);
  });
  const smartPdf = document.getElementById("smartPdf");
  const pdfMode = document.getElementById("pdfMode");
  smartPdf.checked = opts.smartPdf;
  pdfMode.value = opts.pdfMode;
  smartPdf.addEventListener("change", () => chrome.storage.local.set({ smartPdf: smartPdf.checked }).then(saved));
  pdfMode.addEventListener("change", () => chrome.storage.local.set({ pdfMode: pdfMode.value }).then(saved));
  document.getElementById("pdfDebugger").addEventListener("click", () => {
    chrome.storage.local.set({ smartPdf: true, pdfMode: "advanced" });
    smartPdf.checked = true;
    pdfMode.value = "advanced";
    const status = document.getElementById("pdfStatus");
    chrome.permissions.request({ permissions: ["debugger"] }, (granted) => {
      if (chrome.runtime.lastError || !granted) {
        status.textContent = "Advanced detection needs the optional debugger permission.";
        return;
      }
      chrome.tabs.query({ active: true, currentWindow: true }, ([tab]) => {
        if (!tab || !tab.id) { status.textContent = "No active tab found."; return; }
        chrome.runtime.sendMessage({ type: "swiftget-pdf-debugger", enabled: true, tabId: tab.id }, (res) => {
          status.textContent = chrome.runtime.lastError ? chrome.runtime.lastError.message : (res && res.ok ? "Advanced detection attached to this tab." : (res && res.error) || "Could not attach.");
        });
      });
    });
  });
  document.getElementById("pdfDebuggerStop").addEventListener("click", () => {
    chrome.tabs.query({ active: true, currentWindow: true }, ([tab]) => {
      const status = document.getElementById("pdfStatus");
      if (!tab || !tab.id) { status.textContent = "No active tab found."; return; }
      chrome.runtime.sendMessage({ type: "swiftget-pdf-debugger", enabled: false, tabId: tab.id }, (res) => {
        status.textContent = chrome.runtime.lastError ? chrome.runtime.lastError.message : (res && res.ok ? "Advanced detection stopped." : (res && res.error) || "Could not stop.");
      });
    });
  });
}

init();
