import activateCoreSpork from './activateCoreSpork.js';

// Removed in Core v24, which rejects their names
const REMOVED_SPORKS = [
  'SPORK_3_INSTANTSEND_BLOCK_FILTERING',
  'SPORK_9_SUPERBLOCKS_ENABLED',
];

/**
 * Activate the given sporks, skipping those Core v24 removed when this Core lacks them
 *
 * @typedef activateKnownCoreSporks
 * @param {RpcClient} rpcClient
 * @param {string[]} sporks
 * @returns {Promise<string[]>} activated sporks
 */
export default async function activateKnownCoreSporks(rpcClient, sporks) {
  const { result: knownSporks } = await rpcClient.spork('show');

  const activatedSporks = sporks.filter((spork) => (
    Object.hasOwn(knownSporks, spork) || !REMOVED_SPORKS.includes(spork)
  ));

  await Promise.all(
    activatedSporks.map((spork) => activateCoreSpork(rpcClient, spork)),
  );

  return activatedSporks;
}
