import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Read-only inventory/link check. Historical records stay intact and are
// reported separately from current integration continuation documents.
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const roots = ['aidlc-docs', 'okc-core/aidlc-docs', 'okc-hooks/aidlc-docs', 'okc-mcp/aidlc-docs', 'okc-web/aidlc-docs'];
const inventory = [];
const issues = [];
function files(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
    const full = path.join(dir, entry.name);
    return entry.isDirectory() ? files(full) : full.endsWith('.md') ? [full] : [];
  });
}
for (const scope of roots) {
  const documents = files(path.join(root, scope));
  let links = 0;
  for (const file of documents) {
    const original = fs.readFileSync(file, 'utf8');
    const prose = original.replace(/^(`{3,}|~{3,})[^\n]*\n[\s\S]*?^\1\s*$/gm, '');
    for (const match of prose.matchAll(/\[[^\]\n]+\]\((<?[^)\n]+>?)\)/g)) {
      let target = match[1].replace(/^<|>$/g, '').split('#')[0];
      if (!target || /^[a-z][a-z0-9+.-]*:/i.test(target) || target.startsWith('/api/')) continue;
      try { target = decodeURIComponent(target); } catch { /* literal malformed historical URL */ }
      links++;
      if (!fs.existsSync(path.resolve(path.dirname(file), target))) {
        issues.push({ file: path.relative(root, file), target });
      }
    }
  }
  inventory.push({ scope, documents: documents.length, linksChecked: links });
}
const current = issue => issue.file.startsWith('aidlc-docs/')
  || /continuous-sync|integration-sync-repair|versioned-serving|web-knowledge/.test(issue.file);
const currentIssues = issues.filter(current);
const drafts = issue => issue.file.endsWith('.draft.md');
console.log(JSON.stringify({ inventory, currentIssues,
  historicalIssues: issues.filter(issue => !current(issue) && !drafts(issue)),
  draftReferences: issues.filter(drafts),
}, null, 2));
if (currentIssues.length) process.exitCode = 1;
