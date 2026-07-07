// Seed the contract from an app's test plan (the iOS SwiftExampleApp/TEST_PLAN.md
// §4 catalog). Seeds the app/tier/category lookup documents first, then one
// testCase per plan row with integer foreign keys (app/tier/category codes).
//
// Idempotent: lookups are keyed by `code`, testCases by the unique (testId, app).
// Existing docs are skipped by default; --update replaces changed testCases.
//
// Usage:
//   QA_IDENTITY_ID=... QA_PRIVATE_KEY=... node src/seed.mjs
//   ... node src/seed.mjs --app SwiftExampleApp --ids CORE-01,ID-04 --update
//   ... node src/seed.mjs --tier Essential --category Identity --limit 10

import { parseArgs } from 'node:util';
import { randomBytes } from 'node:crypto';
import { loadDotEnv, connect, loadOwnerAuth, readConfig } from './sdk.mjs';
import { parseTestPlan, resolvePlanCommit, DEFAULT_TEST_PLAN } from './parse-test-plan.mjs';
import {
  APPS, TIERS, CATEGORIES, DEFAULT_APP, appCode, tierCode, categoryCode, checkTags,
} from './codes.mjs';

const CONTENT_FIELDS = [
  'testId', 'app', 'tier', 'category', 'title', 'layer', 'implStatus',
  'description', 'entryPoint', 'prerequisites', 'planCommit', 'tags',
];

function entropy() { return Uint8Array.from(randomBytes(32)); }

function testCaseProps(row, app) {
  const props = {
    testId: row.testId,
    app,
    tier: tierCode(row.tier),
    category: categoryCode(row.category),
    title: row.title,
    layer: row.layer,
    implStatus: row.implStatus,
  };
  for (const f of ['description', 'entryPoint', 'prerequisites', 'planCommit']) {
    if (row[f]) props[f] = row[f];
  }
  // DPP document schemas don't support typed arrays, so tags are stored as a
  // comma-separated string (the dashboard splits on ',').
  const tags = checkTags(row.tags, row.testId);
  if (tags.length) props.tags = tags.join(',');
  return props;
}

function pickContent(json) {
  const out = {};
  for (const f of CONTENT_FIELDS) if (json?.[f] !== undefined) out[f] = json[f];
  return out;
}

function contentEquals(a, b) {
  return CONTENT_FIELDS.every((f) => (a?.[f] ?? undefined) === (b?.[f] ?? undefined));
}

function csv(v) { return v ? v.split(',').map((s) => s.trim()).filter(Boolean) : undefined; }

async function findOne(sdk, contractId, documentTypeName, where) {
  const res = await sdk.documents.query({
    dataContractId: contractId, documentTypeName, where, limit: 1,
  });
  for (const doc of res.values()) if (doc) return doc;
  return undefined;
}

// Ensure the app/tier/category lookup documents exist (keyed by `code`).
async function seedLookups(sdk, Document, contractId, ownerId, signer, identityKey) {
  let created = 0; let skipped = 0;
  for (const [type, rows] of [['app', APPS], ['tier', TIERS], ['category', CATEGORIES]]) {
    for (const row of rows) {
      if (await findOne(sdk, contractId, type, [['code', '==', row.code]])) { skipped += 1; continue; }
      const props = { code: row.code, name: row.name };
      if (row.platform) props.platform = row.platform;
      if (row.description) props.description = row.description;
      const doc = new Document({
        ownerId, dataContractId: contractId, documentTypeName: type, properties: props, entropy: entropy(),
      });
      await sdk.documents.create({ document: doc, identityKey, signer });
      created += 1;
      console.log(`  + ${type} ${row.code} = ${row.name}`);
    }
  }
  console.log(`Lookups: ${created} created, ${skipped} skipped.`);
}

async function main() {
  loadDotEnv();
  const { values } = parseArgs({
    options: {
      app: { type: 'string' },
      plan: { type: 'string' },
      ids: { type: 'string' },
      tier: { type: 'string' },
      category: { type: 'string' },
      limit: { type: 'string' },
      update: { type: 'boolean', default: false },
    },
  });

  const appName = values.app || DEFAULT_APP;
  const app = appCode(appName);
  const planPath = values.plan || DEFAULT_TEST_PLAN;

  const { sdk, mod, network } = await connect();
  const cfg = readConfig(network);
  if (!cfg?.contractId) throw new Error(`No contract registered for ${network}. Run register.mjs first.`);
  const contractId = cfg.contractId;
  console.log(`Connected to ${network}. Contract ${contractId}. App '${appName}' (code ${app}).`);

  const { ownerId, signer, identityKey } = await loadOwnerAuth(sdk, mod, network);
  const { Document } = mod;

  await seedLookups(sdk, Document, contractId, ownerId, signer, identityKey);

  const planCommit = resolvePlanCommit(planPath);
  let rows = parseTestPlan(planPath, planCommit);

  // Apply selection filters first so retired-cleanup and seeding act on the same scope.
  const idFilter = csv(values.ids);
  const tierFilter = csv(values.tier)?.map((s) => s.toLowerCase());
  const catFilter = csv(values.category)?.map((s) => s.toLowerCase());
  if (idFilter) rows = rows.filter((r) => idFilter.includes(r.testId));
  if (tierFilter) rows = rows.filter((r) => tierFilter.includes(r.tier.toLowerCase()));
  if (catFilter) rows = rows.filter((r) => catFilter.includes(r.category.toLowerCase()));

  // Retired (➖) rows are historical markers in the plan, not runnable tests. Drop
  // them from the upsert AND delete any already-seeded testCase for that
  // (testId, app), so the on-chain catalog / dashboard never carries a confusing
  // "Unspecified / Unknown, no runs" entry (e.g. DOC-09, folded into DOC-02).
  const retired = rows.filter((r) => (r.implStatus || '').trim() === '➖');
  rows = rows.filter((r) => (r.implStatus || '').trim() !== '➖');
  let deleted = 0; let retiredFailed = 0;
  for (const r of retired) {
    try {
      const existing = await findOne(sdk, contractId, 'testCase', [['testId', '==', r.testId], ['app', '==', app]]);
      if (!existing) continue;
      await sdk.documents.delete({
        document: {
          id: String(existing.toJSON().$id), ownerId, dataContractId: contractId, documentTypeName: 'testCase',
        },
        identityKey,
        signer,
      });
      deleted += 1;
      console.log(`  - retired ${r.testId} (deleted on-chain)`);
    } catch (e) {
      retiredFailed += 1;
      console.error(`  ! retired ${r.testId} delete failed: ${e?.message || e}`);
    }
  }
  if (retired.length) {
    console.log(`Retired (➖): ${retired.length} in scope (${retired.map((r) => r.testId).join(', ')}); ${deleted} deleted, ${retiredFailed} failed.`);
  }

  if (values.limit !== undefined) {
    const limit = Number(values.limit);
    if (!Number.isInteger(limit) || limit <= 0) {
      throw new Error(`--limit must be a positive integer (got '${values.limit}').`);
    }
    rows = rows.slice(0, limit);
  }

  console.log(`Plan commit ${planCommit ?? 'unknown'}; seeding ${rows.length} testCase row(s).`);

  let created = 0; let updated = 0; let skipped = 0; let failed = 0;
  for (const row of rows) {
    const props = testCaseProps(row, app);
    try {
      const existing = await findOne(sdk, contractId, 'testCase', [['testId', '==', row.testId], ['app', '==', app]]);
      if (existing) {
        const existingJson = existing.toJSON();
        // Carry forward fields the new row doesn't set (e.g. planCommit when git
        // history is unavailable) so --update never silently drops provenance.
        const merged = { ...pickContent(existingJson), ...props };
        if (!values.update || contentEquals(existingJson, merged)) { skipped += 1; continue; }
        const doc = new Document({
          id: String(existingJson.$id),
          ownerId,
          dataContractId: contractId,
          documentTypeName: 'testCase',
          properties: merged,
          revision: BigInt(existingJson.$revision ?? 1) + 1n,
        });
        await sdk.documents.replace({ document: doc, identityKey, signer });
        updated += 1;
        console.log(`  ~ updated ${row.testId}`);
      } else {
        const doc = new Document({
          ownerId, dataContractId: contractId, documentTypeName: 'testCase', properties: props, entropy: entropy(),
        });
        await sdk.documents.create({ document: doc, identityKey, signer });
        created += 1;
        console.log(`  + created ${row.testId}`);
      }
    } catch (e) {
      failed += 1;
      console.error(`  ! ${row.testId} failed: ${e?.message || e}`);
    }
  }

  console.log(`\nSeed complete: ${created} created, ${updated} updated, ${skipped} skipped, `
    + `${deleted} retired-deleted, ${failed + retiredFailed} failed.`);
  if (failed || retiredFailed) process.exit(1);
}

main().catch((e) => { console.error('seed failed:', e?.stack || e); process.exit(1); });
