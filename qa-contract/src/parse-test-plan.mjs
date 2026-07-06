// Parse the §4 catalog tables of SwiftExampleApp/TEST_PLAN.md into testCase rows.
//
// Each catalog row looks like:
//   | CORE-05 | Send Core L1 transaction | Core | Essential | ✅ |  | `SendTransactionView` ... |
// Columns: ID | Action(title) | Layer | Tier | Status(implStatus) | Tags | Entry point & notes
// Tags is a comma-separated, lowercase cell (often empty); attached to the row only when non-empty.
// The Category/Domain is NOT a column — it comes from the section header, e.g.
//   ### 4.1 Core / Wallet — `Domain=Core`
// Only content between "## 4." and "## 5." is parsed.

import { readFileSync, existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { QA_DIR } from './sdk.mjs';

export const REPO_ROOT = resolve(QA_DIR, '..');
export const DEFAULT_TEST_PLAN = join(
  REPO_ROOT,
  'packages',
  'swift-sdk',
  'SwiftExampleApp',
  'TEST_PLAN.md',
);

// Resolve the TEST_PLAN commit to stamp on records: $PLAN_COMMIT, else the git
// short-sha of the plan file, else undefined.
export function resolvePlanCommit(planPath = DEFAULT_TEST_PLAN) {
  if (process.env.PLAN_COMMIT) return process.env.PLAN_COMMIT.trim();
  try {
    return execFileSync('git', ['log', '-1', '--format=%h', '--', planPath], {
      cwd: REPO_ROOT, encoding: 'utf8',
    }).trim() || undefined;
  } catch { return undefined; }
}

const TIERS = ['Essential', 'Common', 'Thorough', 'Uncommon', 'Manual'];
const ID_RE = /^[A-Z][A-Z0-9]*-\d+$/;

function normalizeTier(raw) {
  const t = (raw || '').trim();
  const hit = TIERS.find((x) => x.toLowerCase() === t.toLowerCase());
  return hit || 'Unspecified';
}

function splitRow(line) {
  // Split on pipes that are not escaped (\|), then unescape within each cell.
  const cells = line.split(/(?<!\\)\|/).map((c) => c.replaceAll('\\|', '|').trim());
  // A markdown row starts and ends with '|', producing empty first/last cells.
  if (cells.length && cells[0] === '') cells.shift();
  if (cells.length && cells[cells.length - 1] === '') cells.pop();
  return cells;
}

function isSeparator(cells) {
  return cells.length > 0 && cells.every((c) => /^:?-{2,}:?$/.test(c));
}

function firstCodeToken(notes) {
  const m = notes.match(/`([^`]+)`/);
  return m ? m[1] : '';
}

function truncate(s, max) {
  if (s == null) return undefined;
  const str = String(s);
  if (str.length <= max) return str;
  return `${str.slice(0, max - 1)}…`;
}

export function parseTestPlan(planPath = DEFAULT_TEST_PLAN, planCommit) {
  if (!existsSync(planPath)) throw new Error(`TEST_PLAN not found at ${planPath}`);
  const lines = readFileSync(planPath, 'utf8').split('\n');

  const rows = [];
  let inCatalog = false;
  let currentCategory;

  for (const line of lines) {
    const trimmed = line.trim();

    // Section bounds: enter at "## 4.", leave at "## 5.".
    if (/^##\s+4\./.test(trimmed)) { inCatalog = true; continue; }
    if (/^##\s+5\./.test(trimmed)) { inCatalog = false; continue; }
    if (!inCatalog) continue;

    // Track the current Domain/Category from section headers.
    const dom = trimmed.match(/Domain\s*=\s*([A-Za-z]+)/);
    if (dom) { currentCategory = dom[1]; continue; }
    if (trimmed.startsWith('#')) continue;

    if (!trimmed.startsWith('|')) continue;
    const cells = splitRow(trimmed);
    // Catalog rows are 7 cells: ID | Action | Layer | Tier | Status | Tags | Notes.
    // Require all 7 so a short row can't shift Notes into the Tags cell.
    if (cells.length < 7) continue;
    if (isSeparator(cells)) continue;

    const [testId, title, layer, tier, status, tagsCell, ...rest] = cells;
    if (!ID_RE.test(testId)) continue; // header row or non-catalog row

    const tags = (tagsCell || '')
      .split(',')
      .map((s) => s.trim().toLowerCase())
      .filter(Boolean);
    const notes = rest.join(' | ').trim();
    rows.push({
      testId,
      title: truncate(title, 255),
      tier: normalizeTier(tier),
      category: currentCategory || 'Unknown',
      layer: layer || 'Unknown',
      implStatus: truncate(status || '?', 32),
      description: truncate(notes, 2048),
      entryPoint: truncate(firstCodeToken(notes), 512) || undefined,
      ...(tags.length ? { tags } : {}),
      ...(planCommit ? { planCommit: truncate(planCommit, 64) } : {}),
    });
  }
  return rows;
}

// Allow running directly for a quick sanity check: `node src/parse-test-plan.mjs`
if (import.meta.url === `file://${process.argv[1]}`) {
  const rows = parseTestPlan();
  console.log(`Parsed ${rows.length} catalog rows.`);
  const byCat = {};
  for (const r of rows) byCat[r.category] = (byCat[r.category] || 0) + 1;
  console.log('By category:', byCat);
  console.log('First 3:', JSON.stringify(rows.slice(0, 3), null, 2));
}
