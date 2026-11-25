import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parseTestPlan, testPlanFor } from '../src/parse-test-plan.mjs';

function planWith(...rows) {
  const path = join(mkdtempSync(join(tmpdir(), 'qa-plan-')), 'TEST_PLAN.md');
  writeFileSync(path, [
    '## 4. Catalog',
    '### 4.1 Contracts — `Domain=Contract`',
    '| ID | Action | Layer | Tier | Status | Tags | Entry point & notes |',
    '|---|---|---|---|---|---|---|',
    ...rows,
    '## 5. After the catalog',
    '| ZZ-01 | Outside the catalog | Platform | Common | ✅ |  | ignored |',
  ].join('\n'));
  return path;
}

test('should keep a row whose id ends in a lowercase letter', () => {
  const rows = parseTestPlan(planWith(
    '| DC-11 | Umbrella | Platform | Uncommon | ➖ |  | split |',
    '| DC-11a | Ban an identity | Platform | Uncommon | ✅ |  | `moderate.send` |',
    '| DC-11d | Propose a deletion | Platform | Uncommon | ✅ | proposal | notes |',
  ));
  assert.deepEqual(rows.map((r) => r.testId), ['DC-11', 'DC-11a', 'DC-11d']);
  assert.equal(rows[1].category, 'Contract');
  assert.equal(rows[1].entryPoint, 'moderate.send');
  assert.deepEqual(rows[2].tags, ['proposal']);
});

test('should refuse a catalog row whose id it does not recognise', () => {
  const plan = planWith('| dc-12 | Lowercase prefix | Platform | Common | ✅ |  | notes |');
  assert.throws(() => parseTestPlan(plan), /Unrecognised test id 'dc-12'/);
});

test('should parse every catalog row of both apps\' plans', () => {
  for (const app of ['SwiftExampleApp', 'KotlinExampleApp']) {
    assert.ok(parseTestPlan(testPlanFor(app)).length > 100, app);
  }
});
