// Read back documents and verify the contract's indices. Read-only: no identity
// or private key required.
//
// Usage:
//   node src/query.mjs                                   # self-check: exercises every index
//   node src/query.mjs --type app                        # list apps (or tier / category)
//   node src/query.mjs --type testCase --app SwiftExampleApp --tier Essential
//   node src/query.mjs --type testCase --testId CORE-05
//   node src/query.mjs --type testRun --testId CORE-05            # owner+app+test, newest first
//   node src/query.mjs --type testRun --testId CORE-05 --result pass
//   node src/query.mjs --type testRun --buildRef 45fdf33901
//   add --proof for a verified Platform proof, --json for raw output.
//
// tier/category/app are integer foreign keys; this tool resolves them to names
// via src/codes.mjs for display. testRun indices are $ownerId-prefixed, so
// non-buildRef testRun queries scope to the contract owner + app (+ a testId).

import { parseArgs } from 'node:util';
import { loadDotEnv, connect, readConfig, networkId } from './sdk.mjs';
import {
  appCode, tierCode, categoryCode, DEFAULT_APP,
  APP_BY_CODE, TIER_BY_CODE, CATEGORY_BY_CODE,
} from './codes.mjs';

const RESOLVE = { app: APP_BY_CODE, tier: TIER_BY_CODE, category: CATEGORY_BY_CODE };

function toPlain(map) {
  const out = [];
  for (const doc of map.values()) if (doc) out.push(doc.toJSON());
  return out;
}

async function run(sdk, query, proof) {
  const res = proof
    ? (await sdk.documents.queryWithProof(query)).data
    : await sdk.documents.query(query);
  return toPlain(res);
}

function printDocs(label, docs, fields) {
  console.log(`\n# ${label} (${docs.length})`);
  for (const d of docs) {
    const parts = fields.map((f) => {
      const v = d[f] ?? d[`$${f}`] ?? '';
      const name = RESOLVE[f]?.[v];
      return name !== undefined ? `${f}=${v}(${name})` : `${f}=${JSON.stringify(v)}`;
    });
    console.log(`  ${parts.join('  ')}`);
  }
}

async function selfCheck(sdk, contractId, ownerId, app, netId, limit, proof) {
  console.log('Index self-check — each query below requires the named index to succeed.\n');
  const q = (documentTypeName, where, orderBy) => run(sdk, {
    dataContractId: contractId, documentTypeName, where, ...(orderBy ? { orderBy } : {}), limit,
  }, proof);

  // --- lookup doc types: byCode, byName ---
  for (const [type, code, name] of [['app', 0, 'SwiftExampleApp'], ['tier', 0, 'Essential'], ['category', 1, 'Identity']]) {
    printDocs(`${type} index 'byCode'  where code == ${code}`, await q(type, [['code', '==', code]]), ['code', 'name']);
    printDocs(`${type} index 'byName'  where name == ${name}`, await q(type, [['name', '==', name]]), ['code', 'name']);
  }

  // --- testCase indices: testIdApp (unique), appTier, appCategory ---
  const tcFields = ['testId', 'app', 'tier', 'category', 'title'];
  printDocs("testCase index 'testIdApp'  where testId == CORE-05, app == 0",
    await q('testCase', [['testId', '==', 'CORE-05'], ['app', '==', app]]), tcFields);
  printDocs("testCase index 'appTier'  where app == 0, tier == 0 (Essential)",
    await q('testCase', [['app', '==', app], ['tier', '==', 0]]), tcFields);
  printDocs("testCase index 'appCategory'  where app == 0, category == 1 (Identity)",
    await q('testCase', [['app', '==', app], ['category', '==', 1]]), tcFields);

  // --- testRun indices (all $ownerId-prefixed except buildRefOwner) ---
  const trFields = ['testId', 'app', 'result', 'network', 'buildRef', 'createdAt'];
  const base = [['$ownerId', '==', ownerId], ['app', '==', app], ['testId', '==', 'CORE-05']];
  const desc = [['$createdAt', 'desc']];
  // ownerAppTestNetworkCreated also serves the equality-only [$ownerId,app,testId,network] prefix.
  printDocs(`testRun index 'ownerAppTestNetworkCreated'  $ownerId, app==0, testId==CORE-05, network==${netId} order $createdAt desc`,
    await q('testRun', [...base, ['network', '==', netId]], desc), trFields);
  printDocs("testRun index 'ownerAppTestResultCreated'  + result==pass order $createdAt desc",
    await q('testRun', [...base, ['result', '==', 'pass']], desc), trFields);
  printDocs("testRun index 'ownerAppTestCreated'  $ownerId, app==0, testId==CORE-05 order $createdAt desc",
    await q('testRun', base, desc), trFields);
  printDocs("testRun index 'buildRefOwner'  buildRef==45fdf33901, $ownerId==owner",
    await q('testRun', [['buildRef', '==', '45fdf33901'], ['$ownerId', '==', ownerId]]), trFields);

  console.log('\n✅ All 13 indexed queries returned without error — indices are valid.');
}

async function main() {
  loadDotEnv();
  const { values } = parseArgs({
    options: {
      type: { type: 'string' },
      app: { type: 'string' },
      testId: { type: 'string' },
      tier: { type: 'string' },
      category: { type: 'string' },
      result: { type: 'string' },
      network: { type: 'string' },
      buildRef: { type: 'string' },
      limit: { type: 'string', default: '50' },
      proof: { type: 'boolean', default: false },
      json: { type: 'boolean', default: false },
    },
  });
  const limit = Number(values.limit);
  if (!Number.isInteger(limit) || limit <= 0) {
    throw new Error(`--limit must be a positive integer (got '${values.limit}').`);
  }

  const { sdk, network } = await connect();
  const cfg = readConfig(network);
  if (!cfg?.contractId) throw new Error(`No contract registered for ${network}. Run register.mjs first.`);
  const contractId = cfg.contractId;
  const ownerId = cfg.ownerId;
  const netId = networkId(network);
  const app = appCode(values.app || DEFAULT_APP);
  console.log(`Connected to ${network}. Contract ${contractId}${values.proof ? ' (proof-verified)' : ''}.`);

  if (!values.type) { await selfCheck(sdk, contractId, ownerId, app, netId, limit, values.proof); return; }

  const where = []; const orderBy = [];
  if (['app', 'tier', 'category'].includes(values.type)) {
    orderBy.push(['code', 'asc']); // byCode index
  } else if (values.type === 'testCase') {
    // Indices are testIdApp / appTier / appCategory — each pairs app with exactly
    // one of testId/tier/category, so reject combinations no index can serve.
    if ([values.testId, values.tier, values.category].filter(Boolean).length > 1) {
      throw new Error('Use only one of --testId / --tier / --category for testCase (no app+tier+category index).');
    }
    where.push(['app', '==', app]);
    if (values.testId) where.push(['testId', '==', values.testId]);
    if (values.tier) where.push(['tier', '==', tierCode(values.tier)]);
    if (values.category) where.push(['category', '==', categoryCode(values.category)]);
  } else if (values.type === 'testRun') {
    // testRun indices are either buildRef-led (buildRefOwner) or $ownerId,app,testId-led.
    // Reject combinations no index can serve so the failure is a clear CLI error.
    if (values.buildRef) {
      if (values.testId || values.result || values.network) {
        throw new Error('--buildRef uses the buildRefOwner index; it cannot be combined with --testId/--result/--network.');
      }
      where.push(['buildRef', '==', values.buildRef]);
      if (ownerId) where.push(['$ownerId', '==', ownerId]);
    } else {
      if (!values.testId) {
        throw new Error('testRun queries need --testId (with optional --result OR --network), or --buildRef.');
      }
      if (values.network && values.result) {
        throw new Error('--network and --result can\'t be combined for testRun (no covering index; use one).');
      }
      if (ownerId) where.push(['$ownerId', '==', ownerId]);
      where.push(['app', '==', app]);
      where.push(['testId', '==', values.testId]);
      if (values.network) where.push(['network', '==', networkId(values.network)]);
      if (values.result) where.push(['result', '==', values.result]);
      orderBy.push(['$createdAt', 'desc']);
    }
  } else {
    throw new Error("--type must be one of: app, tier, category, testCase, testRun.");
  }

  const query = { dataContractId: contractId, documentTypeName: values.type, limit };
  if (where.length) query.where = where;
  if (orderBy.length) query.orderBy = orderBy;

  const docs = await run(sdk, query, values.proof);
  if (values.json) { console.log(JSON.stringify(docs, null, 2)); return; }
  const fields = {
    app: ['code', 'name', 'platform'],
    tier: ['code', 'name'],
    category: ['code', 'name'],
    testCase: ['testId', 'app', 'tier', 'category', 'title', 'layer', 'implStatus'],
    testRun: ['testId', 'app', 'result', 'network', 'buildRef', 'device', 'createdAt'],
  }[values.type];
  printDocs(`${values.type} results`, docs, fields);
}

main().catch((e) => { console.error('query failed:', e?.stack || e); process.exit(1); });
