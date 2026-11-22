// Migrate historical testRun records from the previous QA contract (v5, whose
// runs only its owner could record) onto the freshly-registered contract, where
// listed admins record them. testIds pass through the remap table in codes.mjs,
// which leaves v5 ids as they are.
//
// Reading the old contract is public (no key); writing the new runs uses the
// identity from .env (QA_IDENTITY_ID / QA_PRIVATE_KEY), which must be an admin
// of the new contract (register.mjs lists the owner). Migrated runs get a FRESH
// $createdAt and are owned by that identity: the original run time is not
// preserved. They are written oldest first, so their order is.
//
// Run AFTER register.mjs + seed.mjs (the new testCases must already exist), once
// per app (--app). Idempotent: a run already present for the same
// (testId, result, buildRef) is skipped, so re-running won't duplicate.
//
// Usage:
//   QA_IDENTITY_ID=... QA_PRIVATE_KEY=... node src/migrate-runs.mjs [--dry-run]
//     [--from <oldContractId>] [--from-owner <oldOwnerId>] [--app SwiftExampleApp]

import { parseArgs } from 'node:util';
import { randomBytes } from 'node:crypto';
import { loadDotEnv, connect, loadOwnerAuth, readConfig } from './sdk.mjs';
import { appCode, DEFAULT_APP, remapTestId } from './codes.mjs';

// Previous (v5) QA contract on testnet + its owner identity (public, read-only).
const DEFAULT_OLD_CONTRACT = '9tshSfq5TU3gAEJCVa6KWxmb7cS7kkzRSmSSTmteKGcU';
const DEFAULT_OLD_OWNER = '2YBjQKEP6APyu9JADk46yoELL2aKAcRW7yCLTCV2d4dD';

// Enumerate old testCases across a generous category code range so none are
// missed regardless of the old numbering.
const OLD_CATEGORY_CODES = Array.from({ length: 16 }, (_, i) => i);

function entropy() { return Uint8Array.from(randomBytes(32)); }

const PAGE = 100;

// Page through every match, not just the first PAGE, using `startAfter` on the
// last document's $id. `documents.query` caps a page at PAGE rows, so a full
// page means there may be more. Callers that can exceed PAGE rows (the per-test
// run history + idempotency lookup) pass a stable `orderBy` so paging is
// deterministic; the equality-only testCase scan is bounded well under PAGE.
async function queryAll(sdk, dataContractId, documentTypeName, where, orderBy) {
  const out = [];
  let startAfter;
  for (;;) {
    const res = await sdk.documents.query({
      dataContractId,
      documentTypeName,
      where,
      ...(orderBy ? { orderBy } : {}),
      ...(startAfter ? { startAfter } : {}),
      limit: PAGE,
    });
    const raw = [...res.values()];
    for (const doc of raw) if (doc) out.push(doc.toJSON());
    if (raw.length < PAGE) break;
    const lastId = raw[raw.length - 1]?.toJSON().$id;
    if (!lastId) break; // can't advance the cursor — stop rather than loop forever
    startAfter = lastId;
  }
  return out;
}

async function main() {
  loadDotEnv();
  const { values } = parseArgs({
    options: {
      'dry-run': { type: 'boolean', default: false },
      from: { type: 'string' },
      'from-owner': { type: 'string' },
      app: { type: 'string' },
    },
  });

  const dryRun = values['dry-run'];
  const appName = values.app || DEFAULT_APP;
  const app = appCode(appName);
  const oldContractId = (values.from || process.env.OLD_CONTRACT_ID || DEFAULT_OLD_CONTRACT).trim();

  let { sdk, mod, network } = await connect();
  if (network !== 'testnet') {
    console.warn(`Warning: network is '${network}', but the default old contract is a testnet id.`);
  }

  const cfg = readConfig(network);
  if (!cfg?.contractId) throw new Error(`No contract registered for ${network}. Run register.mjs first.`);
  const newContractId = cfg.contractId;
  if (newContractId === oldContractId) {
    throw new Error(`Refusing to migrate: the registered contract (${newContractId}) equals --from. Register the new contract first (register.mjs --force).`);
  }

  // Resolve the old owner (the old contract's testRun indices are $ownerId-prefixed).
  let oldOwner = (values['from-owner'] || process.env.OLD_OWNER_ID || '').trim();
  if (!oldOwner) {
    try {
      const oldContract = await sdk.contracts.fetch(oldContractId);
      oldOwner = oldContract ? String(oldContract.ownerId) : '';
    } catch { /* fall through to default */ }
  }
  if (!oldOwner) oldOwner = DEFAULT_OLD_OWNER;

  let { ownerId: newOwner, signer, identityKey } = await loadOwnerAuth(sdk, mod, network);
  let { Document } = mod;

  // Reconnect a fresh SDK + re-derive auth. A fresh instance re-fetches the
  // identity-contract nonce from chain, recovering from the SDK's
  // "identity nonce too far in future" drift that cascades during bulk creates.
  async function reconnect() {
    ({ sdk, mod, network } = await connect());
    ({ ownerId: newOwner, signer, identityKey } = await loadOwnerAuth(sdk, mod, network));
    ({ Document } = mod);
  }

  console.log('Migrating runs:');
  console.log(`  from  ${oldContractId} (owner ${oldOwner})`);
  console.log(`  to    ${newContractId} (owner ${newOwner})`);
  console.log(`  app '${appName}' (code ${app})${dryRun ? '  [DRY RUN]' : ''}`);

  // 1. Enumerate old testIds (across every old category code so MW/GRP are caught).
  const oldTestIds = new Set();
  for (const c of OLD_CATEGORY_CODES) {
    const cases = await queryAll(sdk, oldContractId, 'testCase', [['app', '==', app], ['category', '==', c]]);
    for (const tc of cases) if (tc.testId) oldTestIds.add(tc.testId);
  }
  console.log(`Found ${oldTestIds.size} old testCase ids.`);

  // 2. For each old testId, read its runs and re-create them under the new id.
  let migrated = 0; let skipped = 0; let failed = 0; let runsSeen = 0;
  for (const oldId of oldTestIds) {
    const newId = remapTestId(oldId);
    const oldRuns = await queryAll(
      sdk, oldContractId, 'testRun',
      [['$ownerId', '==', oldOwner], ['app', '==', app], ['testId', '==', oldId]],
      [['$createdAt', 'asc']],
    );
    if (!oldRuns.length) continue;

    // Idempotency: skip a run already present for (newId, result, buildRef),
    // whichever admin recorded it.
    const existing = await queryAll(
      sdk, newContractId, 'testRun',
      [['app', '==', app], ['testId', '==', newId]],
      [['$createdAt', 'asc']],
    );
    const present = new Set(existing.map((r) => `${r.result}|${r.buildRef}`));

    for (const run of oldRuns) {
      runsSeen += 1;
      const label = oldId === newId ? newId : `${oldId}->${newId}`;
      if (present.has(`${run.result}|${run.buildRef}`)) { skipped += 1; continue; }

      const props = { testId: newId, app, result: run.result, network: run.network, buildRef: run.buildRef };
      for (const f of ['device', 'evidence', 'notes', 'blockerReason']) {
        if (run[f] !== undefined && run[f] !== null && run[f] !== '') props[f] = run[f];
      }
      if (dryRun) {
        console.log(`  [dry] ${label}  ${run.result}  ${run.buildRef}`);
        migrated += 1;
        continue;
      }
      for (let attempt = 1; attempt <= 6; attempt += 1) {
        try {
          const doc = new Document({
            ownerId: newOwner, dataContractId: newContractId, documentTypeName: 'testRun', properties: props, entropy: entropy(),
          });
          await sdk.documents.create({ document: doc, identityKey, signer });
          migrated += 1;
          console.log(`  + ${label}  ${run.result}  ${run.buildRef}`);
          break;
        } catch (e) {
          const msg = e?.message || String(e);
          // Recover from identity-contract-nonce drift: a fresh SDK re-syncs
          // the nonce from chain. Retry non-fatally up to a few times.
          if (/nonce/i.test(msg) && attempt < 6) {
            console.error(`  ~ ${label} nonce desync (attempt ${attempt}) — reconnecting + retrying`);
            await new Promise((r) => setTimeout(r, 2500));
            await reconnect();
            // A nonce error can be a false negative — the write may have landed
            // before the ack was lost. Re-check before retrying so we don't
            // create a duplicate testRun for the same (testId, result, buildRef).
            const after = await queryAll(
              sdk, newContractId, 'testRun',
              [['app', '==', app], ['testId', '==', newId]],
              [['$createdAt', 'asc']],
            );
            if (after.some((r) => `${r.result}|${r.buildRef}` === `${run.result}|${run.buildRef}`)) {
              console.log(`  = ${label} (${run.result}/${run.buildRef}) already landed — skipping retry`);
              skipped += 1;
              break;
            }
            continue;
          }
          failed += 1;
          console.error(`  ! ${label} (${run.result}/${run.buildRef}) failed: ${msg}`);
          break;
        }
      }
    }
  }

  console.log(`\nRun migration ${dryRun ? '(dry run) ' : ''}complete: ${runsSeen} source runs; `
    + `${migrated} ${dryRun ? 'would migrate' : 'migrated'}, ${skipped} skipped (already present), ${failed} failed.`);
  if (failed) process.exit(1);
}

main().catch((e) => { console.error('migrate-runs failed:', e?.stack || e); process.exit(1); });
