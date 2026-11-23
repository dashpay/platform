// Read back documents and verify the contract's indices. Read-only: no identity
// or private key required.
//
// Usage:
//   node src/query.mjs                                   # self-check: exercises every index
//   node src/query.mjs --type app                        # list apps (or tier / category)
//   node src/query.mjs --type testCase --app SwiftExampleApp --tier Essential
//   node src/query.mjs --type testCase --testId CORE-05
//   node src/query.mjs --type admin                      # who may record runs
//   node src/query.mjs --type testRun --testId CORE-05            # app+test, newest first
//   node src/query.mjs --type testRun --testId CORE-05 --result pass
//   node src/query.mjs --type testRun --buildRef 45fdf33901
//   node src/query.mjs --type testRun --recordedBy <identityId>   # one admin's runs
//   add --proof for a verified Platform proof, --json for raw output.
//
// tier/category/app are integer foreign keys; this tool resolves them to names
// via src/codes.mjs for display. Several admins record runs, so testRun queries
// scope to an app + a testId whoever recorded them; --recordedBy narrows to one.

import { parseArgs } from 'node:util';
import {
  loadDotEnv, connect, readConfig, networkId, requireCurrentSchema,
} from './sdk.mjs';
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

  // --- admin index: byIdentity (unique; the one testRun.ownerRefersTo reads) ---
  printDocs("admin index 'byIdentity'  where identity == contract owner",
    await q('admin', [['identity', '==', ownerId]]), ['identity', 'label']);

  // --- testRun indices (app,testId-led, plus buildRefCreated and ownerCreated) ---
  const trFields = ['testId', 'app', 'result', 'network', 'buildRef', 'ownerId', 'createdAt'];
  const base = [['app', '==', app], ['testId', '==', 'CORE-05']];
  const desc = [['$createdAt', 'desc']];
  // appTestNetworkCreated also serves the equality-only [app,testId,network] prefix.
  printDocs(`testRun index 'appTestNetworkCreated'  app==0, testId==CORE-05, network==${netId} order $createdAt desc`,
    await q('testRun', [...base, ['network', '==', netId]], desc), trFields);
  printDocs("testRun index 'appTestResultCreated'  + result==pass order $createdAt desc",
    await q('testRun', [...base, ['result', '==', 'pass']], desc), trFields);
  printDocs("testRun index 'appTestCreated'  app==0, testId==CORE-05 order $createdAt desc",
    await q('testRun', base, desc), trFields);
  printDocs("testRun index 'buildRefCreated'  buildRef==45fdf33901 order $createdAt desc",
    await q('testRun', [['buildRef', '==', '45fdf33901']], desc), trFields);
  printDocs("testRun index 'ownerCreated'  $ownerId==contract owner order $createdAt desc",
    await q('testRun', [['$ownerId', '==', ownerId]], desc), trFields);

  console.log('\n✅ All 15 indexed queries returned without error — indices are valid.');
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
      recordedBy: { type: 'string' },
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

  // The self-check and the admin and testRun reads depend on the committed
  // schema's types and indices; the lookups and testCase read the same on the
  // previous contract.
  if (!values.type || ['admin', 'testRun'].includes(values.type)) requireCurrentSchema(cfg, network);

  if (!values.type) { await selfCheck(sdk, contractId, ownerId, app, netId, limit, values.proof); return; }

  const where = []; const orderBy = [];
  if (['app', 'tier', 'category'].includes(values.type)) {
    orderBy.push(['code', 'asc']); // byCode index
  } else if (values.type === 'admin') {
    // The whole list: it is short, and byIdentity serves a lookup of one.
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
    // testRun indices are buildRef-led (buildRefCreated), $ownerId-led (ownerCreated)
    // or app,testId-led. Reject combinations no index can serve so the failure is
    // a clear CLI error.
    if (values.buildRef || values.recordedBy) {
      // Neither index holds `app`, and a page filtered afterwards would drop
      // matching runs past the limit, so an explicit --app is refused too.
      if (values.app || values.testId || values.result || values.network
        || (values.buildRef && values.recordedBy)) {
        throw new Error('--buildRef and --recordedBy each use an index of their own; '
          + 'neither can be combined with the other or with --app/--testId/--result/--network.');
      }
      if (values.buildRef) where.push(['buildRef', '==', values.buildRef]);
      else where.push(['$ownerId', '==', values.recordedBy]);
      orderBy.push(['$createdAt', 'desc']);
    } else {
      if (!values.testId) {
        throw new Error('testRun queries need --testId (with optional --result OR --network), --buildRef or --recordedBy.');
      }
      if (values.network && values.result) {
        throw new Error('--network and --result can\'t be combined for testRun (no covering index; use one).');
      }
      where.push(['app', '==', app]);
      where.push(['testId', '==', values.testId]);
      if (values.network) where.push(['network', '==', networkId(values.network)]);
      if (values.result) where.push(['result', '==', values.result]);
      orderBy.push(['$createdAt', 'desc']);
    }
  } else {
    throw new Error("--type must be one of: admin, app, tier, category, testCase, testRun.");
  }

  const query = { dataContractId: contractId, documentTypeName: values.type, limit };
  if (where.length) query.where = where;
  if (orderBy.length) query.orderBy = orderBy;

  const docs = await run(sdk, query, values.proof);
  if (values.json) { console.log(JSON.stringify(docs, null, 2)); return; }
  const fields = {
    admin: ['identity', 'label', 'createdAt'],
    app: ['code', 'name', 'platform'],
    tier: ['code', 'name'],
    category: ['code', 'name'],
    testCase: ['testId', 'app', 'tier', 'category', 'title', 'layer', 'implStatus'],
    testRun: ['testId', 'app', 'result', 'network', 'buildRef', 'device', 'ownerId', 'createdAt'],
  }[values.type];
  printDocs(`${values.type} results`, docs, fields);
}

main().catch((e) => { console.error('query failed:', e?.stack || e); process.exit(1); });
