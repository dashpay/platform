// Read back testCase / testRun documents and verify the contract's indices.
// Read-only: no identity or private key required.
//
// Usage:
//   node src/query.mjs                       # self-check: exercises every index
//   node src/query.mjs --type testCase --tier Essential
//   node src/query.mjs --type testCase --category Identity --limit 5
//   node src/query.mjs --type testRun --testId CORE-05            # owner+test, newest first
//   node src/query.mjs --type testRun --testId CORE-05 --result pass
//   node src/query.mjs --type testRun --testId CORE-05 --network testnet
//   node src/query.mjs --type testRun --buildRef 45fdf33901
//   add --proof to fetch with a verified Platform proof, --json for raw output.
//   testRun indices are $ownerId-prefixed, so non-buildRef queries scope to the
//   contract owner (read from contract-id.<network>.json) and need a --testId.

import { parseArgs } from 'node:util';
import {
  loadDotEnv, connect, readConfig, networkId,
} from './sdk.mjs';

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

async function selfCheck(sdk, contractId, ownerId, netId, limit, proof) {
  console.log('Index self-check — each query below requires the named index to succeed.\n');

  // --- testCase indices: testId (unique), tier, category ---
  let docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testCase',
    where: [['testId', '==', 'CORE-05']], limit: 1,
  }, proof);
  printDocs("testCase index 'testId'  where testId == CORE-05", docs, ['testId', 'title', 'tier', 'implStatus']);

  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testCase',
    where: [['tier', '==', 'Essential']], orderBy: [['tier', 'asc']], limit,
  }, proof);
  printDocs("testCase index 'tier'  where tier == Essential", docs, ['testId', 'tier', 'category']);

  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testCase',
    where: [['category', '==', 'Identity']], orderBy: [['category', 'asc']], limit,
  }, proof);
  printDocs("testCase index 'category'  where category == Identity", docs, ['testId', 'category', 'tier']);

  // --- testRun indices (all $ownerId-prefixed) ---
  const trFields = ['testId', 'result', 'network', 'buildRef', 'createdAt'];

  // ownerTestNetwork: $ownerId, testId, network
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['$ownerId', '==', ownerId], ['testId', '==', 'CORE-05'], ['network', '==', netId]], limit,
  }, proof);
  printDocs(`testRun index 'ownerTestNetwork'  $ownerId==owner, testId==CORE-05, network==${netId}`, docs, trFields);

  // ownerTestNetworkCreated: $ownerId, testId, network, $createdAt
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['$ownerId', '==', ownerId], ['testId', '==', 'CORE-05'], ['network', '==', netId]],
    orderBy: [['$createdAt', 'desc']], limit,
  }, proof);
  printDocs("testRun index 'ownerTestNetworkCreated'  + order $createdAt desc", docs, trFields);

  // ownerTestResultCreated: $ownerId, testId, result, $createdAt
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['$ownerId', '==', ownerId], ['testId', '==', 'CORE-05'], ['result', '==', 'pass']],
    orderBy: [['$createdAt', 'desc']], limit,
  }, proof);
  printDocs("testRun index 'ownerTestResultCreated'  $ownerId==owner, testId==CORE-05, result==pass order $createdAt desc", docs, trFields);

  // ownerTestCreated: $ownerId, testId, $createdAt
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['$ownerId', '==', ownerId], ['testId', '==', 'CORE-05']],
    orderBy: [['$createdAt', 'desc']], limit,
  }, proof);
  printDocs("testRun index 'ownerTestCreated'  $ownerId==owner, testId==CORE-05 order $createdAt desc", docs, trFields);

  // buildRefOwner: buildRef, $ownerId
  docs = await run(sdk, {
    dataContractId: contractId, documentTypeName: 'testRun',
    where: [['buildRef', '==', '45fdf33901'], ['$ownerId', '==', ownerId]], limit,
  }, proof);
  printDocs("testRun index 'buildRefOwner'  buildRef==45fdf33901, $ownerId==owner", docs, trFields);

  console.log('\n✅ All 8 indexed queries returned without error — indices are valid.');
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
  console.log(`Connected to ${network}. Contract ${contractId}${values.proof ? ' (proof-verified)' : ''}.`);

  if (!values.type) { await selfCheck(sdk, contractId, ownerId, netId, limit, values.proof); return; }

  const where = []; const orderBy = [];
  if (values.type === 'testCase') {
    if (values.testId) where.push(['testId', '==', values.testId]);
    if (values.tier) { where.push(['tier', '==', values.tier]); orderBy.push(['tier', 'asc']); }
    if (values.category) { where.push(['category', '==', values.category]); orderBy.push(['category', 'asc']); }
  } else if (values.type === 'testRun') {
    // testRun indices are $ownerId-prefixed (except buildRefOwner). Query by buildRef
    // alone uses buildRefOwner; otherwise scope to the owner + testId per the
    // owner/test/{network,result}/$createdAt indices.
    if (values.buildRef) {
      where.push(['buildRef', '==', values.buildRef]);
      if (ownerId) where.push(['$ownerId', '==', ownerId]);
    } else {
      if (ownerId) where.push(['$ownerId', '==', ownerId]);
      if (values.testId) where.push(['testId', '==', values.testId]);
      if (values.network) where.push(['network', '==', networkId(values.network)]);
      if (values.result) where.push(['result', '==', values.result]);
      orderBy.push(['$createdAt', 'desc']);
    }
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
