/**
 * @typedef activateCoreSpork
 * @param {RpcClient} rpcClient
 * @param {string} spork
 * @returns {Promise<void>}
 */
export async function activateCoreSpork(rpcClient, spork) {
  await rpcClient.sporkupdate(spork, 0);
}
