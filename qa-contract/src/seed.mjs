// Seed testCase documents from SwiftExampleApp/TEST_PLAN.md §4 catalog.
//
// Idempotent: each row is keyed by its unique testId. Existing testCases are
// skipped by default; pass --update to replace ones whose content changed.
//
// Usage:
//   QA_IDENTITY_ID=... QA_PRIVATE_KEY=... node src/seed.mjs
//   ... node src/seed.mjs --ids CORE-01,ID-04 --update
//   ... node src/seed.mjs --tier Essential --category Identity --limit 10

import { parseArgs } from 'node:util';
import { randomBytes } from 'node:crypto';
import {
  loadDotEnv, connect, loadOwnerAuth, readConfig,
} from './sdk.mjs';
import { parseTestPlan, resolvePlanCommit, DEFAULT_TEST_PLAN } from './parse-test-plan.mjs';

const CONTENT_FIELDS = [
  'testId', 'title', 'tier', 'category', 'layer', 'implStatus',
  'description', 'entryPoint', 'prerequisites', 'planCommit',
];

function cleanProps(row) {
  const props = {};
  for (const f of CONTENT_FIELDS) {
    if (row[f] !== undefined && row[f] !== null && row[f] !== '') props[f] = row[f];
  }
  return props;
}

function contentEquals(existing, props) {
  return CONTENT_FIELDS.every((f) => (existing?.[f] ?? undefined) === (props[f] ?? undefined));
}

function csv(v) { return v ? v.split(',').map((s) => s.trim()).filter(Boolean) : undefined; }

async function findExisting(sdk, contractId, testId) {
  const res = await sdk.documents.query({
    dataContractId: contractId,
    documentTypeName: 'testCase',
    where: [['testId', '==', testId]],
    limit: 1,
  });
  for (const doc of res.values()) if (doc) return doc;
  return undefined;
}

async function main() {
  loadDotEnv();
  const { values } = parseArgs({
    options: {
      plan: { type: 'string' },
      ids: { type: 'string' },
      tier: { type: 'string' },
      category: { type: 'string' },
      limit: { type: 'string' },
      update: { type: 'boolean', default: false },
    },
  });

  const planPath = values.plan || DEFAULT_TEST_PLAN;
  const { sdk, mod, network } = await connect();
  const cfg = readConfig(network);
  if (!cfg?.contractId) throw new Error(`No contract registered for ${network}. Run register.mjs first.`);
  const contractId = cfg.contractId;
  console.log(`Connected to ${network}. Contract ${contractId}.`);

  const { ownerId, signer, identityKey } = await loadOwnerAuth(sdk, mod, network);

  const planCommit = resolvePlanCommit(planPath);
  let rows = parseTestPlan(planPath, planCommit);

  const idFilter = csv(values.ids);
  const tierFilter = csv(values.tier)?.map((s) => s.toLowerCase());
  const catFilter = csv(values.category)?.map((s) => s.toLowerCase());
  if (idFilter) rows = rows.filter((r) => idFilter.includes(r.testId));
  if (tierFilter) rows = rows.filter((r) => tierFilter.includes(r.tier.toLowerCase()));
  if (catFilter) rows = rows.filter((r) => catFilter.includes(r.category.toLowerCase()));
  if (values.limit) rows = rows.slice(0, Number(values.limit));

  console.log(`Plan commit ${planCommit ?? 'unknown'}; seeding ${rows.length} testCase row(s).`);

  const { Document } = mod;
  let created = 0; let updated = 0; let skipped = 0; let failed = 0;

  for (const row of rows) {
    const props = cleanProps(row);
    try {
      const existing = await findExisting(sdk, contractId, row.testId);
      if (existing) {
        const existingJson = existing.toJSON();
        if (!values.update) { skipped += 1; continue; }
        if (contentEquals(existingJson, props)) { skipped += 1; continue; }
        const doc = new Document({
          id: String(existingJson.$id),
          ownerId,
          dataContractId: contractId,
          documentTypeName: 'testCase',
          properties: props,
          revision: BigInt(existingJson.$revision ?? 1) + 1n,
        });
        await sdk.documents.replace({ document: doc, identityKey, signer });
        updated += 1;
        console.log(`  ~ updated ${row.testId}`);
      } else {
        const doc = new Document({
          ownerId,
          dataContractId: contractId,
          documentTypeName: 'testCase',
          properties: props,
          entropy: Uint8Array.from(randomBytes(32)),
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

  console.log(`\nSeed complete: ${created} created, ${updated} updated, ${skipped} skipped, ${failed} failed.`);
  if (failed) process.exit(1);
}

main().catch((e) => { console.error('seed failed:', e?.stack || e); process.exit(1); });
