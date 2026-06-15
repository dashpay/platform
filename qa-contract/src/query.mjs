// Read back testCase / testRun documents and verify the contract's indices.
// Read-only: no identity or private key required.
//
// Usage:
//   node src/query.mjs                       # self-check: exercises every index
//   node src/query.mjs --type testCase --tier Essential
//   node src/query.mjs --type testCase --category Identity --limit 5
//   node src/query.mjs --type testRun --testId CORE-05
//   node src/query.mjs --type testRun --result pass
//   node src/query.mjs --type testRun --buildRef 45fdf33901
//   add --proof to fetch with a verified Platform proof, --json for raw output.

import { parseArgs } from 'node:util';
import { loadDotEnv, connect, readConfig } from './sdk.mjs';

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
    const parts = fields.map((f) => `${f}=${JSON.stringify(d[f] ?? d[`$${f}`] ?? '')}`);
    console.log(`  ${parts.join('  ')}`);
  }
}

async function selfCheck(sdk, contractId, limit, proof) {
  console.log('Index self-check — each query below requires the named index to succeed.\n');

  // testCase.testId (unique)
  let docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testCase',
    where: [['testId', '==', 'CORE-05']], limit: 1,
  }, proof);
  printDocs("testCase index 'testId'  where testId == CORE-05", docs, ['testId', 'title', 'tier', 'implStatus']);

  // testCase.tier
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testCase',
    where: [['tier', '==', 'Essential']], orderBy: [['tier', 'asc']], limit,
  }, proof);
  printDocs("testCase index 'tier'  where tier == Essential", docs, ['testId', 'tier', 'category']);

  // testCase.category
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testCase',
    where: [['category', '==', 'Identity']], orderBy: [['category', 'asc']], limit,
  }, proof);
  printDocs("testCase index 'category'  where category == Identity", docs, ['testId', 'category', 'tier']);

  // testRun.testIdCreatedAt
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['testId', '==', 'CORE-05']], orderBy: [['$createdAt', 'desc']], limit,
  }, proof);
  printDocs("testRun index 'testIdCreatedAt'  where testId == CORE-05 order $createdAt desc", docs,
    ['testId', 'result', 'buildRef', 'createdAt']);

  // testRun.resultCreatedAt
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['result', '==', 'pass']], orderBy: [['$createdAt', 'desc']], limit,
  }, proof);
  printDocs("testRun index 'resultCreatedAt'  where result == pass order $createdAt desc", docs,
    ['testId', 'result', 'buildRef']);

  // testRun.buildRef
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['buildRef', '==', '45fdf33901']], limit,
  }, proof);
  printDocs("testRun index 'buildRef'  where buildRef == 45fdf33901", docs,
    ['testId', 'result', 'buildRef']);

  console.log('\n✅ All 6 indexed queries returned without error — indices are valid.');
}

async function main() {
  loadDotEnv();
  const { values } = parseArgs({
    options: {
      type: { type: 'string' },
      testId: { type: 'string' },
      tier: { type: 'string' },
      category: { type: 'string' },
      result: { type: 'string' },
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
  console.log(`Connected to ${network}. Contract ${contractId}${values.proof ? ' (proof-verified)' : ''}.`);

  if (!values.type) { await selfCheck(sdk, contractId, limit, values.proof); return; }

  const where = []; const orderBy = [];
  if (values.type === 'testCase') {
    if (values.testId) where.push(['testId', '==', values.testId]);
    if (values.tier) { where.push(['tier', '==', values.tier]); orderBy.push(['tier', 'asc']); }
    if (values.category) { where.push(['category', '==', values.category]); orderBy.push(['category', 'asc']); }
  } else if (values.type === 'testRun') {
    if (values.testId) { where.push(['testId', '==', values.testId]); orderBy.push(['$createdAt', 'desc']); }
    if (values.result) { where.push(['result', '==', values.result]); orderBy.push(['$createdAt', 'desc']); }
    if (values.buildRef) where.push(['buildRef', '==', values.buildRef]);
  } else {
    throw new Error("--type must be 'testCase' or 'testRun'.");
  }

  const query = { dataContractId: contractId, documentTypeName: values.type, limit };
  if (where.length) query.where = where;
  if (orderBy.length) query.orderBy = orderBy;

  const docs = await run(sdk, query, values.proof);
  if (values.json) { console.log(JSON.stringify(docs, null, 2)); return; }
  const fields = values.type === 'testCase'
    ? ['testId', 'title', 'tier', 'category', 'layer', 'implStatus']
    : ['testId', 'result', 'network', 'buildRef', 'device', 'createdAt'];
  printDocs(`${values.type} results`, docs, fields);
}

main().catch((e) => { console.error('query failed:', e?.stack || e); process.exit(1); });
