// Unit tests for the non-blocking response classifier.
const fs = require("fs"), path = require("path"), vm = require("vm"), assert = require("assert");
const root = { URL };
vm.runInNewContext(fs.readFileSync(path.join(__dirname, "..", "pdf-detector.js"), "utf8"), root);
const detector = root.IRQPdfDetector;
const event = (url, mime, extra = {}) => detector.classify({
  requestId: "r1", tabId: 2, statusCode: 200, url,
  responseHeaders: [{ name: "content-type", value: mime }, ...(extra.headers || [])], ...extra,
});

assert(event("https://x.test/report.pdf?token=1", "application/octet-stream"), "pdf extension with query");
assert(event("https://x.test/get?id=1", "application/pdf"), "PDF MIME without extension");
assert(event("https://x.test/get", "application/octet-stream", { headers: [{ name: "Content-Disposition", value: "attachment; filename=paper.pdf" }] }), "disposition filename");
assert(event("https://x.test/get", "application/octet-stream", { headers: [{ name: "Content-Disposition", value: "attachment; filename*=UTF-8''paper%20one.pdf" }] }), "RFC 5987 filename");
assert.strictEqual(event("https://x.test/get", "application/octet-stream"), null, "do not classify generic binary as PDF");
assert.strictEqual(event("https://x.test/report.pdf", "text/html"), null, "do not classify login HTML as PDF");
const range = event("https://x.test/get?part=1", "application/pdf", { statusCode: 206, responseHeaders: [{ name: "Content-Type", value: "application/pdf" }, { name: "Content-Range", value: "bytes 0-99/1000" }] });
assert(range.partial && range.statusCode === 206 && range.size === 1000, "206 range response uses whole-file size");
detector.redirected({ requestId: "r1", tabId: 2, url: "https://publisher.test/download", redirectUrl: "https://cdn.test/paper" });
const final = detector.classify({ requestId: "r1", tabId: 2, statusCode: 200, url: "https://cdn.test/paper?signature=x", responseHeaders: [{ name: "Content-Type", value: "application/pdf" }] });
assert.strictEqual(final.originalUrl, "https://publisher.test/download", "keep pre-redirect URL");
assert(final.redirectChain.includes("https://cdn.test/paper"), "retain redirect chain");
console.log("pdf-detector: all checks passed");
