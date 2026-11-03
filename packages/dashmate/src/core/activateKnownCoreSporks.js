import activateCoreSpork from './activateCoreSpork.js';

/**
 * Activate the given sporks that this Core version still has
 *
 * Core v24 removed SPORK_3_INSTANTSEND_BLOCK_FILTERING and SPORK_9_SUPERBLOCKS_ENABLED
 * and rejects their names, so sporks missing from `spork show` are skipped.
 *
 * @typedef activateKnownCoreSporks
 * @param {RpcClient} rpcClient
 * @param {string[]} sporks
 * @returns {Promise<string[]>} activated sporks
 */
export default async function activateKnownCoreSporks(rpcClient, sporks) {
  const { result: knownSporks } = await rpcClient.spork('show');

  const activatedSporks = sporks.filter((spork) => Object.hasOwn(knownSporks, spork));

  await Promise.all(
    activatedSporks.map((spork) => activateCoreSpork(rpcClient, spork)),
  );

  return activatedSporks;
}
