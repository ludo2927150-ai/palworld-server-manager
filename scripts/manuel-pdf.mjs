// Génère docs/MANUEL.pdf à partir de docs/MANUEL.md (Markdown → HTML → PDF via Chromium/Playwright).
// Usage : node scripts/manuel-pdf.mjs   (Playwright doit être installé : npm i -D playwright, ou PLAYWRIGHT_MODULE=/chemin/vers/playwright)
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { Marked } from "marked";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const exe = process.env.CHROMIUM_PATH || undefined;

const slug = (t) => t.toLowerCase().replace(/<[^>]+>/g, "").replace(/[^\p{L}\p{N}\s_-]/gu, "").trim().replace(/\s/g, "-");
const marked = new Marked({ gfm: true });
marked.use({ renderer: {
  heading(text, depth) { const plain = text.replace(/<[^>]+>/g, "").replace(/&#39;/g, "'").replace(/&quot;/g, '"').replace(/&amp;/g, "&"); return `<h${depth} id="${slug(plain)}">${text}</h${depth}>`; },
  html() { return ""; },
  link(href, _t, inner) { return href.startsWith("#") ? `<a href="${href}">${inner}</a>` : `<span class="ext">${inner}</span>`; },
} });

const md = readFileSync("docs/MANUEL.md", "utf8").replace(/^# .*\n/, ""); // le titre H1 devient la page de garde
const body = marked.parse(md);
const date = new Date().toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });

const html = `<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Palworld Server Manager — Manuel d'utilisation</title><style>
@page { size: A4; margin: 20mm 16mm 18mm; }
* { box-sizing: border-box; }
body { font-family: "Segoe UI", "DejaVu Sans", Arial, sans-serif; font-size: 10pt; line-height: 1.55; color: #0f172a; }
.cover { height: 250mm; display: flex; flex-direction: column; justify-content: center; page-break-after: always; border-left: 6px solid #14b8a6; padding-left: 14mm; }
.cover h1 { font-size: 30pt; margin: 0 0 6mm; color: #0f766e; }
.cover p { font-size: 13pt; margin: 0 0 3mm; color: #334155; }
.cover small { color: #64748b; margin-top: 12mm; font-size: 10pt; }
h1, h2, h3, h4 { page-break-after: avoid; }
h2 { font-size: 17pt; color: #0f766e; border-bottom: 2px solid #99f6e4; padding-bottom: 2mm; margin: 0 0 4mm; page-break-before: always; }
h2:first-of-type { page-break-before: auto; }
h3 { font-size: 12.5pt; color: #115e59; margin: 6mm 0 2mm; }
h4 { font-size: 10.5pt; margin: 4mm 0 1mm; }
p, ul, ol, table, blockquote, pre { margin: 2mm 0; }
ul, ol { padding-left: 6mm; } li { margin: 0.8mm 0; }
a { color: #0f766e; text-decoration: none; }
code { font-family: "Cascadia Mono", "DejaVu Sans Mono", monospace; font-size: 8.8pt; background: #f1f5f9; padding: 0.3mm 1mm; border-radius: 1mm; }
pre { background: #f1f5f9; padding: 3mm; border-radius: 2mm; overflow: hidden; white-space: pre-wrap; } pre code { background: none; padding: 0; }
blockquote { border-left: 3px solid #f59e0b; background: #fffbeb; padding: 2mm 4mm; margin-left: 0; }
table { width: 100%; border-collapse: collapse; font-size: 9pt; }
th { background: #ccfbf1; text-align: left; } th, td { border: 0.3mm solid #cbd5e1; padding: 1.2mm 2mm; vertical-align: top; }
tr { page-break-inside: avoid; }
hr { border: 0; border-top: 0.3mm solid #cbd5e1; margin: 5mm 0; }
.ext { color: #475569; }
</style></head><body>
<section class="cover"><h1>Palworld Server Manager</h1><p>Manuel d'utilisation complet</p><p>Toutes les fonctions, réglages, automatismes, recettes et dépannage</p><small>Version de test — ${date}</small></section>
${body}
</body></html>`;

writeFileSync("/tmp/manuel.html", html);
const browser = await chromium.launch(exe ? { executablePath: exe } : {});
const page = await browser.newPage();
await page.setContent(html, { waitUntil: "load" });
await page.pdf({
  path: "docs/MANUEL.pdf", format: "A4", printBackground: true, displayHeaderFooter: true,
  headerTemplate: "<div></div>",
  footerTemplate: `<div style="font-size:8px;color:#64748b;width:100%;padding:0 16mm;display:flex;justify-content:space-between"><span>Palworld Server Manager — Manuel d'utilisation</span><span><span class="pageNumber"></span> / <span class="totalPages"></span></span></div>`,
  margin: { top: "20mm", bottom: "18mm", left: "16mm", right: "16mm" },
});
await browser.close();
console.log("docs/MANUEL.pdf généré");
