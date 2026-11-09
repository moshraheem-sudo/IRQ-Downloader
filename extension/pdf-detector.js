// Optional document response classifier used by the existing network listener.
// It does not intercept or block requests; all downloads still go through IRQ.
(function (root) {
  "use strict";

  const redirects = new Map();
  const MAX_TRACKED = 300;

  function headersMap(headers) {
    const out = Object.create(null);
    for (const h of headers || []) out[String(h.name || "").toLowerCase()] = h.value || "";
    return out;
  }

  function filenameFromDisposition(value) {
    const star = /filename\*\s*=\s*(?:UTF-8''|"?)([^;"\s]+)/i.exec(value || "");
    if (star) {
      try { return decodeURIComponent(star[1].replace(/"$/, "")); } catch (_) { return star[1]; }
    }
    const plain = /filename\s*=\s*(?:"([^"]+)"|([^;\s]+))/i.exec(value || "");
    return plain ? (plain[1] || plain[2]) : "";
  }

  function classify(details) {
    const h = headersMap(details && details.responseHeaders);
    const mime = (h["content-type"] || "").split(";")[0].trim().toLowerCase();
    const disposition = h["content-disposition"] || "";
    const name = filenameFromDisposition(disposition);
    let path = "";
    try { path = new URL(details.url).pathname; } catch (_) {}
    const pdfMime = mime === "application/pdf" || mime === "application/x-pdf";
    const pdfName = /\.pdf$/i.test(name);
    const pdfPath = /\.pdf$/i.test(path);
    const isGeneric = /^(application\/octet-stream|binary\/octet-stream)$/.test(mime);
    // An octet-stream is accepted only with explicit PDF filename evidence.
    if (!(pdfMime || pdfName || (pdfPath && !/^text\/html$/.test(mime)))) return null;
    if (isGeneric && !pdfName && !pdfPath) return null;
    if (/^(text\/html|application\/xhtml\+xml)$/.test(mime)) return null;
    const prior = redirects.get(details.requestId);
    const rangeTotal = /\/([0-9]+|\*)\s*$/.exec(h["content-range"] || "");
    const size = rangeTotal ? (rangeTotal[1] === "*" ? 0 : Number(rangeTotal[1])) : Number(h["content-length"]) || 0;
    return {
      url: details.url,
      originalUrl: prior && prior.originalUrl || details.url,
      redirectChain: prior && prior.chain || [],
      mime,
      filename: name,
      contentDisposition: disposition,
      size,
      statusCode: details.statusCode || 0,
      partial: details.statusCode === 206 || /^bytes\s/i.test(h["content-range"] || ""),
      requestId: details.requestId,
      tabId: details.tabId,
      frameId: details.frameId,
      initiator: details.initiator || "",
      type: "PDF document",
      ext: "pdf",
      isDocument: true,
      source: "network_headers",
    };
  }

  function redirected(details) {
    if (!details || details.tabId < 0) return;
    let item = redirects.get(details.requestId);
    if (!item) item = { originalUrl: details.url, chain: [] };
    item.chain.push(details.url, details.redirectUrl);
    if (item.chain.length > 20) item.chain.splice(0, item.chain.length - 20);
    redirects.delete(details.requestId);
    redirects.set(details.requestId, item);
    while (redirects.size > MAX_TRACKED) redirects.delete(redirects.keys().next().value);
  }

  function clear(requestId) { redirects.delete(requestId); }

  root.IRQPdfDetector = Object.freeze({ classify, redirected, clear, filenameFromDisposition });
})(globalThis);
