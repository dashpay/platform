// Submit a single testRun document (an append-only test-execution record).
//
// Usage:
//   QA_IDENTITY_ID=... QA_PRIVATE_KEY=... node src/submit-run.mjs \
//     --testId CORE-05 --result pass --buildRef 45fdf33901 \
//     --device "iPhone 16 (iOS 18.2)" --evidence "txid:30010050…" --notes "..."
//
//   --result must be one of: pass | fail | blocked | skipped
//   --network defaults to $NETWORK (testnet). --blockerReason for blocked/skipped.

import { parseArgs } from 'node:util';
import { randomBytes } from 'node:crypto';
import {
  loadDotEnv, connect, loadOwnerAuth, readConfig, networkId,
} from './sdk.mjs';
import { appCode, DEFAULT_APP } from './codes.mjs';

const RESULTS = ['pass', 'fail', 'blocked', 'skipped'];

async function main() {
  loadDotEnv();
  const { values } = parseArgs({
    options: {
      testId: { type: 'string' },
      app: { type: 'string' },
      result: { type: 'string' },
      buildRef: { type: 'string' },
      network: { type: 'string' },
      device: { type: 'string' },
      evidence: { type: 'string' },
      notes: { type: 'string' },
      blockerReason: { type: 'string' },
      force: { type: 'boolean', default: false },
    },
  });

  const appName = values.app || DEFAULT_APP;
  const app = appCode(appName);
  const testId = values.testId?.trim();
  const result = values.result?.trim().toLowerCase();
  const buildRef = values.buildRef?.trim();
  if (!testId || !result || !buildRef) {
    throw new Error('Required: --testId, --result, --buildRef.');
  }
  if (!RESULTS.includes(result)) {
    throw new Error(`--result must be one of: ${RESULTS.join(', ')} (got '${result}').`);
  }

  // --network selects the target network (so the connection, the loaded contract
  // config, and the stamped properties.network all agree), defaulting to $NETWORK.
  if (values.network?.trim()) process.env.NETWORK = values.network.trim();

  const { sdk, mod, network } = await connect();
  const cfg = readConfig(network);
  if (!cfg?.contractId) throw new Error(`No contract registered for ${network}. Run register.mjs first.`);
  const contractId = cfg.contractId;

  const { ownerId, signer, identityKey } = await loadOwnerAuth(sdk, mod, network);

  // testRun is immutable + non-deletable, so a typo'd testId would create a
  // permanent orphan. Require the matching (testId, app) testCase to exist first.
  if (!values.force) {
    const res = await sdk.documents.query({
      dataContractId: contractId,
      documentTypeName: 'testCase',
      where: [['testId', '==', testId], ['app', '==', app]],
      limit: 1,
    });
    let exists = false;
    for (const d of res.values()) if (d) exists = true;
    if (!exists) {
      throw new Error(`No testCase '${testId}' for app '${appName}' (code ${app}) on ${network}. `
        + 'Seed it first, or pass --force to record the run anyway.');
    }
  }

  const properties = {
    testId, app, result, network: networkId(network), buildRef,
  };
  if (values.device) properties.device = values.device;
  if (values.evidence) properties.evidence = values.evidence;
  if (values.notes) properties.notes = values.notes;
  if (values.blockerReason) properties.blockerReason = values.blockerReason;

  const { Document } = mod;
  const doc = new Document({
    ownerId,
    dataContractId: contractId,
    documentTypeName: 'testRun',
    properties,
    entropy: Uint8Array.from(randomBytes(32)),
  });

  console.log(`Submitting testRun: ${appName}/${testId} = ${result} (build ${buildRef}) on ${network} ...`);
  await sdk.documents.create({ document: doc, identityKey, signer });
  console.log(`✅ testRun recorded for ${appName}/${testId} (${result}).`);
}

main().catch((e) => { console.error('submit-run failed:', e?.stack || e); process.exit(1); });
