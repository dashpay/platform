// Manage the list of identities allowed to record test runs.
//
// testRun declares `ownerRefersTo` into the `admin` document type, so the
// platform accepts a run only from an identity that has an `admin` document.
// Only the contract owner creates and deletes those, so `add` and `remove` must
// be signed by the owner (QA_IDENTITY_ID / QA_PRIVATE_KEY). `list` needs no key.
//
// Usage:
//   node src/admin.mjs list
//   node src/admin.mjs add <identityId> [--label "Sam's laptop"]
//   node src/admin.mjs remove <identityId>
//
// Removing an admin stops new runs from that identity. The runs it already
// recorded stay: testRun is an append-only log.

import { parseArgs } from 'node:util';
import {
  loadDotEnv, connect, loadOwnerAuth, readConfig, findAdmin, addAdmin,
} from './sdk.mjs';

const PAGE = 100;

async function listAdmins(sdk, contractId) {
  const out = [];
  let startAfter;
  for (;;) {
    const res = await sdk.documents.query({
      dataContractId: contractId,
      documentTypeName: 'admin',
      ...(startAfter ? { startAfter } : {}),
      limit: PAGE,
    });
    const raw = [...res.values()].filter(Boolean).map((d) => d.toJSON());
    out.push(...raw);
    if (raw.length < PAGE) break;
    startAfter = raw[raw.length - 1].$id;
  }
  return out;
}

// add/remove are the owner's: refuse early, with the reason, rather than let the
// platform answer with the creation restriction error.
async function loadContractOwnerAuth(sdk, mod, network, cfg) {
  const auth = await loadOwnerAuth(sdk, mod, network);
  if (cfg.ownerId && auth.ownerId !== cfg.ownerId) {
    throw new Error(`Only the contract owner (${cfg.ownerId}) manages admins; `
      + `QA_IDENTITY_ID is ${auth.ownerId}.`);
  }
  return auth;
}

async function main() {
  loadDotEnv();
  const { values, positionals } = parseArgs({
    options: { label: { type: 'string' } },
    allowPositionals: true,
  });
  const [command, identityId] = positionals;
  if (!['list', 'add', 'remove'].includes(command)) {
    throw new Error('Usage: admin.mjs list | add <identityId> [--label <text>] | remove <identityId>');
  }

  const { sdk, mod, network } = await connect();
  const cfg = readConfig(network);
  if (!cfg?.contractId) throw new Error(`No contract registered for ${network}. Run register.mjs first.`);
  const contractId = cfg.contractId;

  if (command === 'list') {
    const admins = await listAdmins(sdk, contractId);
    console.log(`Admins of ${contractId} on ${network} (${admins.length}):`);
    for (const a of admins) {
      const owner = a.identity === cfg.ownerId ? '  (contract owner)' : '';
      console.log(`  ${a.identity}${a.label ? `  ${a.label}` : ''}${owner}`);
    }
    return;
  }

  if (!identityId) throw new Error(`'${command}' needs an identity id.`);
  const { ownerId, signer, identityKey } = await loadContractOwnerAuth(sdk, mod, network, cfg);
  const existing = await findAdmin(sdk, contractId, identityId);

  if (command === 'add') {
    if (existing) { console.log(`${identityId} is already an admin.`); return; }
    await addAdmin(sdk, mod, {
      contractId, ownerId, identityId, label: values.label, signer, identityKey,
    });
    console.log(`✅ ${identityId} can now record runs on ${contractId}.`);
    return;
  }

  if (!existing) { console.log(`${identityId} is not an admin.`); return; }
  await sdk.documents.delete({
    document: {
      id: String(existing.toJSON().$id), ownerId, dataContractId: contractId, documentTypeName: 'admin',
    },
    identityKey,
    signer,
  });
  console.log(`✅ ${identityId} can no longer record runs on ${contractId}.`);
}

main().catch((e) => { console.error('admin failed:', e?.message || e?.stack || e); process.exit(1); });
