const fmtSize = (n) => {
  if (!n) return "";
  const u = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < u.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(i ? 1 : 0)} ${u[i]}`;
};

async function init() {
  document.getElementById("settings").addEventListener("click", () => {
    chrome.runtime.openOptionsPage();
    window.close();
  });

  // the on / off switch: off = the browser keeps its downloads, no buttons
  // on videos, nothing is looked up (the add-in stays installed)
  const on = document.getElementById("enabled");
  const showOn = (v) => {
    on.checked = v;
    document.body.classList.toggle("off", !v);
    document.getElementById("onText").textContent = v ? "IRQ Downloader is on" : "IRQ Downloader is paused";
    document.getElementById("onHint").textContent = v
      ? "Downloads and videos of this browser go to IRQ Downloader"
      : "The browser keeps its downloads, no buttons on videos";
  };
  chrome.storage.local.get({ enabled: true }, (o) => showOn(o.enabled !== false));
  on.addEventListener("change", () => {
    chrome.storage.local.set({ enabled: on.checked });
    showOn(on.checked);
  });

  const look = document.getElementById("look");
  chrome.storage.local.get({ bubbleStyle: "compact" }, (o) => (look.value = o.bubbleStyle || "compact"));
  look.addEventListener("change", () => chrome.storage.local.set({ bubbleStyle: look.value }));

  // how solid the button is on the video (the mouse on it: always solid)
  const solid = document.getElementById("solid");
  const solidText = document.getElementById("solidText");
  const showSolid = (v) => {
    solid.value = String(v);
    solidText.textContent = `${v}%`;
  };
  chrome.storage.local.get({ bubbleOpacity: 0.9 }, (o) => showSolid(Math.round((Number(o.bubbleOpacity) || 0.9) * 100)));
  solid.addEventListener("input", () => {
    showSolid(Number(solid.value));
    chrome.storage.local.set({ bubbleOpacity: Number(solid.value) / 100 });
  });

  // full screen: the button stays out of the way unless the user wants it
  const fullscreen = document.getElementById("fullscreen");
  chrome.storage.local.get({ bubbleFullscreen: false }, (o) => (fullscreen.checked = !!o.bubbleFullscreen));
  fullscreen.addEventListener("change", () => chrome.storage.local.set({ bubbleFullscreen: fullscreen.checked }));

  // size of the "Download this video" button on pages (also in the settings page)
  const scale = document.getElementById("scale");
  const scaleText = document.getElementById("scaleText");
  const OLD = { small: 85, normal: 100, large: 125 };
  const show = (v) => {
    scale.value = String(v);
    scaleText.textContent = `${v}%`;
  };
  chrome.storage.local.get({ bubbleScale: null, bubbleSize: "normal" }, (o) =>
    show(o.bubbleScale != null ? Math.round(o.bubbleScale * 100) : OLD[o.bubbleSize] || 100)
  );
  const save = (v) => {
    show(v);
    chrome.storage.local.set({ bubbleScale: v / 100 });
  };
  scale.addEventListener("input", () => save(Number(scale.value)));
  document.getElementById("scaleReset").addEventListener("click", () => save(100));

  chrome.runtime.sendMessage({ type: "swiftget-status" }, (res) => {
    const el = document.getElementById("status");
    const ok = res && res.ok;
    const nokey = res && res.state === "nokey";
    el.textContent = ok ? "connected" : nokey ? "not connected" : "app not running";
    el.classList.toggle("on", !!ok);
    if (nokey) {
      // the program runs but does not know this add-in (yet): connect again
      const again = document.createElement("button");
      again.className = "act";
      again.textContent = "Connect again";
      again.addEventListener("click", () => {
        again.textContent = "Connecting… (answer in IRQ Downloader if it asks)";
        again.disabled = true;
        chrome.runtime.sendMessage({ type: "swiftget-reconnect" }, (r) => {
          again.disabled = false;
          again.textContent = r && r.ok ? "Connected ✔" : "Not connected - try again";
          if (r && r.ok) {
            el.textContent = "connected";
            el.classList.add("on");
          }
        });
      });
      document.querySelector("section").appendChild(again);
    } else if (!ok) {
      // the program registers "ezoo-download:" when it is installed
      const open = document.createElement("a");
      open.href = "ezoo-download://open";
      open.className = "open";
      open.textContent = "Open IRQ Downloader";
      open.addEventListener("click", () => setTimeout(() => window.close(), 300));
      document.querySelector("section").appendChild(open);
    }
  });

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab) return;
  chrome.runtime.sendMessage({ type: "swiftget-get-media", tabId: tab.id }, (items) => {
    if (!items || !items.length) return;
    const box = document.getElementById("media");
    box.replaceChildren();
    const title = (tab.title || "video").replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_").slice(0, 120);
    // a YouTube playlist: all of its videos in the app's playlist window
    if (/youtube\.com/i.test(tab.url || "") && /[?&]list=/.test(tab.url || "")) {
      const row = document.createElement("div");
      row.className = "media";
      const info = document.createElement("div");
      info.textContent = "All videos of this playlist";
      const btn = document.createElement("button");
      btn.className = "all";
      btn.textContent = "Download all";
      btn.addEventListener("click", () =>
        chrome.runtime.sendMessage({ type: "swiftget-playlist", url: tab.url }, (res) => {
          btn.textContent = res && res.ok ? "Opened ✔" : "App not running";
        })
      );
      row.append(info, btn);
      box.appendChild(row);
    }
    const isAr = (navigator.language || "").startsWith("ar");
    const t = {
      download: isAr ? "تنزيل" : "Download",
      openPage: isAr ? "فتح صفحة المقال" : "Open article page",
      sent: isAr ? "تم الإرسال ✔" : "Sent ✔",
      notRunning: isAr ? "التطبيق غير مشغّل" : "App not running",
      site: isAr ? "الموقع:" : "Site:",
      source: isAr ? "المصدر:" : "Source:",
      type: isAr ? "نوع الاكتشاف:" : "Detection:",
      status: isAr ? "الحالة:" : "Status:",
      size: isAr ? "الحجم:" : "Size:",
      available: isAr ? "متوفر للتحميل" : "Available",
      unknownSize: isAr ? "غير محدد مسبقاً" : "Dynamic / Unknown",
      types: {
        network_headers: isAr ? "استجابة الشبكة (Headers)" : "Network response (Headers)",
        page_metadata: isAr ? "بيانات الصفحة الوصفية (Metadata)" : "Page metadata",
        page_html: isAr ? "رابط داخل الصفحة (HTML)" : "Page document link",
        page_embed: isAr ? "عارض مستندات مضمن (Embed)" : "Embedded viewer",
        debugger_network: isAr ? "كشف متقدم عبر الشبكة (Debugger)" : "Advanced network inspection",
        openalex_open_access: isAr ? "مستودع أبحاث مفتوح (OpenAlex)" : "Open access repository (OpenAlex)",
        unpaywall_open_access: isAr ? "مستودع أبحاث مفتوح (Unpaywall)" : "Open access repository (Unpaywall)",
      },
    };

    items.forEach((m) => {
      if (m.isDocument) {
        const card = document.createElement("div");
        card.className = "doc-card";

        const head = document.createElement("div");
        head.className = "doc-title";
        const icon = document.createElement("span");
        icon.className = "doc-icon";
        icon.textContent = "PDF";
        const titleSpan = document.createElement("span");
        titleSpan.textContent = m.label || "PDF Document";
        head.append(icon, titleSpan);

        let siteHost = "";
        try { siteHost = new URL(m.sourcePage || m.referrer || m.url).hostname; } catch (_) {}

        const meta = document.createElement("div");
        meta.className = "doc-meta";
        meta.innerHTML = `
          <span>${t.site}</span><b>${siteHost || "—"}</b>
          <span>${t.type}</span><b>${t.types[m.source] || m.source || "PDF"}</b>
          <span>${t.status}</span><b>${m.disabled ? (m.sub || "Unavailable") : t.available}</b>
          <span>${t.size}</span><b>${m.size > 0 ? fmtSize(m.size) : t.unknownSize}</b>
        `;

        const actions = document.createElement("div");
        actions.className = "doc-actions";

        const btn = document.createElement("button");
        btn.textContent = t.download;
        btn.disabled = !!m.disabled;
        btn.addEventListener("click", () => {
          chrome.runtime.sendMessage(
            { type: "swiftget-download", item: m, title: tab.title, referrer: tab.url, fileName: `${title}.${m.ext || "pdf"}` },
            (res) => { btn.textContent = res && res.ok ? t.sent : t.notRunning; }
          );
        });
        actions.appendChild(btn);

        const pageUrl = m.sourcePage || m.referrer;
        if (pageUrl && /^https?:/i.test(pageUrl) && pageUrl !== tab.url) {
          const openBtn = document.createElement("button");
          openBtn.className = "btn-sec";
          openBtn.textContent = t.openPage;
          openBtn.addEventListener("click", () => {
            chrome.tabs.create({ url: pageUrl });
          });
          actions.appendChild(openBtn);
        }

        card.append(head, meta, actions);
        box.appendChild(card);
        return;
      }

      const row = document.createElement("div");
      row.className = "media";
      const info = document.createElement("div");
      info.textContent = m.label;
      const small = document.createElement("small");
      small.textContent = m.sub || "";
      info.appendChild(small);
      const btn = document.createElement("button");
      btn.textContent = "Download";
      btn.disabled = !!m.disabled;
      btn.addEventListener("click", () => {
        if (m.resolve) {
          // a stream: its qualities come from the app, shown in its place
          btn.textContent = "…";
          btn.disabled = true;
          chrome.runtime.sendMessage({ type: "swiftget-resolve-video", resolve: m.resolve }, (res) => {
            if (!res || res.error || !res.items) {
              btn.textContent = (res && res.error) || "App not running";
              return;
            }
            const rows = res.items.map((it) => {
              const r = document.createElement("div");
              r.className = "media";
              const inf = document.createElement("div");
              inf.textContent = it.label;
              const go = document.createElement("button");
              go.textContent = "Download";
              go.addEventListener("click", () =>
                chrome.runtime.sendMessage({ type: "swiftget-download", item: it, title: tab.title, referrer: tab.url, fileName: `${title}.${it.ext || "mp4"}` }, (r2) => {
                  go.textContent = r2 && r2.ok ? "Sent ✔" : "App not running";
                })
              );
              r.append(inf, go);
              return r;
            });
            row.replaceWith(...rows);
          });
          return;
        }
        chrome.runtime.sendMessage(
          { type: "swiftget-download", item: m, title: tab.title, referrer: tab.url, fileName: `${title}.${m.ext || "mp4"}` },
          (res) => {
            btn.textContent = res && res.ok ? "Sent ✔" : "App not running";
          }
        );
      });
      row.append(info, btn);
      box.appendChild(row);
    });
  });
}

init();
