// Register the QA data contract on the configured network and write the
// resulting contract ID into qa-contract/contract-id.<network>.json.
//
// Re-runnable: if the committed config already points at a contract that still
// resolves on-network, it is left untouched (use --force to register a fresh
// contract, e.g. after a testnet reset).
//
// Usage:
//   QA_IDENTITY_ID=... QA_PRIVATE_KEY=... node src/register.mjs [--force]

import { parseArgs } from 'node:util';
import {
  loadDotEnv, connect, loadOwnerAuth, loadSchema, schemaSha,
  readConfig, writeConfig,
} from './sdk.mjs';
import { resolvePlanCommit } from './parse-test-plan.mjs';

async function main() {
  loadDotEnv();
  const { values } = parseArgs({ options: { force: { type: 'boolean', default: false } } });

  const { sdk, mod, network } = await connect();
  console.log(`Connected to ${network}.`);

  const currentSchemaSha = schemaSha();

  // Short-circuit if already registered and still resolvable.
  const existing = readConfig(network);
  if (existing?.contractId && !values.force) {
    // fetch() returns undefined for a genuinely absent contract (e.g. testnet
    // reset). A *thrown* error is transient (gRPC/DNS/etc.) and must NOT be
    // mistaken for "gone" — that would publish a duplicate contract. Abort instead.
    let onChain;
    try {
      onChain = await sdk.contracts.fetch(existing.contractId);
    } catch (e) {
      throw new Error(`Could not verify existing contract ${existing.contractId} on ${network}: ${e?.message || e}. `
        + 'Aborting to avoid registering a duplicate — re-run when reachable, or pass --force to deliberately register a fresh contract.');
    }
    if (onChain) {
      if (existing.schemaSha && existing.schemaSha !== currentSchemaSha) {
        throw new Error(
          `Existing contract ${existing.contractId} resolves, but the local schema changed `
          + `(${existing.schemaSha} -> ${currentSchemaSha}). A data contract's schema is immutable, `
          + 'so re-run with --force to publish a fresh contract (new id), or revert the schema.',
        );
      }
      console.log(`Already registered: ${existing.contractId} (still resolves on ${network}).`);
      console.log('Pass --force to register a fresh contract.');
      return;
    }
    console.log(`Config has ${existing.contractId} but it no longer resolves on ${network} `
      + '(testnet reset?). Registering a fresh contract.');
  }

  const { ownerId, identity, signer, identityKey } = await loadOwnerAuth(sdk, mod, network);
  console.log(`Owner identity ${ownerId} (balance ${identity.balance} credits, `
    + `signing key id=${identityKey.keyId ?? identityKey.id} ${identityKey.purpose}/${identityKey.securityLevel}).`);

  const schemas = loadSchema();
  const { DataContract } = mod;
  const dataContract = new DataContract({
    ownerId,
    identityNonce: 0n, // overridden by the SDK with the live identity nonce on publish
    schemas,
    fullValidation: true,
  });

  console.log(`Publishing contract with document types: ${Object.keys(schemas).join(', ')} ...`);
  const published = await sdk.contracts.publish({ dataContract, identityKey, signer });
  const contractId = String(published.id);

  const cfg = {
    network,
    contractId,
    ownerId,
    documentTypes: Object.keys(schemas),
    schemaSha: currentSchemaSha,
    planCommit: resolvePlanCommit() ?? null,
    registeredAt: new Date().toISOString(),
  };
  const path = writeConfig(cfg, network);

  console.log(`\n✅ Registered QA contract on ${network}`);
  console.log(`   contractId: ${contractId}`);
  console.log(`   wrote: ${path}`);
}

main().catch((e) => {
  console.error('register failed:');
  try { console.error('  message :', e?.message); } catch {}
  try { console.error('  toString:', e?.toString?.()); } catch {}
  try { console.error('  String  :', String(e)); } catch {}
  try { console.error('  stack   :', e?.stack); } catch {}
  process.exit(1);
});
