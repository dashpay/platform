/**
 * Re-read seeded state from one specific node, with proofs.
 *
 * This is the assertion that separates "the joiner came up and answers RPC"
 * from "the joiner actually restored the chain's state". Every read goes
 * through the WASM SDK's proved path, so the node must produce a GroveDB proof
 * that carries the value up to a root hash the validator quorum signed. A node
 * that state synced incorrectly cannot fake that; it fails the proof instead
 * of returning plausible-looking data.
 */

import systemIds from '@dashevo/dpns-contract/lib/systemIds.js';
import { getEvoSdk } from './platformSdk.js';

const { contractId: dpnsContractId } = systemIds;

/**
 * Unwrap a `ProofMetadataResponseTyped`, which carries the value alongside the
 * proof and block metadata.
 *
 * @param {*} response
 * @return {*}
 */
function unwrapProved(response) {
  if (response && typeof response === 'object' && 'data' in response) {
    return response.data;
  }

  return response;
}

/**
 * Verify every item a seeding run recorded is readable, and proof-verified,
 * from `config`'s node.
 *
 * @param {Config} config - node to query
 * @param {Config} quorumListConfig
 * @param {Object} manifest - result of seedPlatformState
 * @return {Promise<Object[]>} one result per check
 */
export default async function verifySeededState(config, quorumListConfig, manifest) {
  const { sdk } = await getEvoSdk(config, quorumListConfig);

  const checks = [];

  /**
   * @param {string} name
   * @param {function(): Promise<*>} read
   * @param {function(*): boolean} isPresent
   * @return {Promise<void>}
   */
  const check = async (name, read, isPresent) => {
    try {
      const value = unwrapProved(await read());

      checks.push({
        name,
        present: isPresent(value),
        detail: undefined,
      });
    } catch (error) {
      checks.push({ name, present: false, detail: error.message });
    }
  };

  // Baseline, independent of whether seeding managed to write anything: DPNS
  // is created in the genesis state, so every node that holds a correctly
  // restored Drive can serve it under proof, and a node that restored nothing
  // cannot. This keeps the check meaningful even on a run where seeding was
  // blocked.
  await check(
    `DPNS system data contract ${dpnsContractId}`,
    () => sdk.contracts.fetchWithProof(dpnsContractId),
    (value) => Boolean(value),
  );

  for (const identity of manifest.identities) {
    await check(
      `identity ${identity.id}`,
      () => sdk.identities.fetchWithProof(identity.id),
      (value) => Boolean(value),
    );
  }

  for (const [appName, contract] of Object.entries(manifest.contracts)) {
    await check(
      `data contract ${appName} ${contract.id}`,
      () => sdk.contracts.fetchWithProof(contract.id),
      (value) => Boolean(value),
    );
  }

  for (const document of manifest.documents) {
    await check(
      `document ${document.appName}.${document.documentType} ${document.id}`,
      () => sdk.documents.getWithProof(
        document.contractId,
        document.documentType,
        document.id,
      ),
      (value) => Boolean(value),
    );
  }

  if (manifest.name) {
    await check(
      `DPNS name ${manifest.name.fullName}`,
      () => sdk.dpns.getUsernameByNameWithProof(manifest.name.fullName),
      (value) => Boolean(value),
    );
  }

  // `ranked` rather than `rankedWithProof`: verification is a property of the
  // SDK (built with `proofs: true`), not of the method name. The `WithProof`
  // suffix only decides whether the proof bytes come back to the caller — both
  // variants verify before returning.
  //
  // A ranked index keeps ordered secondary trees beside the index itself.
  // They are part of the snapshot and are not rebuilt by replaying blocks the
  // joiner never saw, so answering a ranked query is a sharper check on the
  // restored state than any plain document read.
  const rankedContract = manifest.contracts.qaRanked;

  if (rankedContract) {
    await check(
      'ranked query over the restored secondary trees',
      () => sdk.documents.ranked({
        dataContractId: rankedContract.id,
        documentTypeName: 'rankedItem',
        groupBy: 'category',
        aggregate: { type: 'count' },
        limit: 5,
      }),
      (value) => Boolean(value && value.entries && value.entries.length > 0),
    );
  }

  return checks;
}

/**
 * Human-readable summary of verification results, for the run log.
 *
 * @param {Object[]} checks
 * @return {string}
 */
export function describeVerification(checks) {
  return checks
    .map(({ name, present, detail }) => (
      `  ${present ? 'present' : 'MISSING'} ${name}${detail ? ` — ${detail}` : ''}`
    ))
    .join('\n');
}
