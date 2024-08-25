/**
 * @typedef activateCoreSpork
 * @param {RpcClient} rpcClient
 * @param {string} spork
 * @param {number} height
 * @returns {Promise<void>}
 */
export default async function activateCoreSpork(rpcClient, spork, height = 0) {
  await rpcClient.sporkupdate(spork, height);
}
