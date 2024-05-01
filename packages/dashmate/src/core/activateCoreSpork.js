/**
 * @typedef activateCoreSpork
 * @param {RpcClient} rpcClient
 * @param {string} spork
 * @returns {Promise<void>}
 */
export default async function activateCoreSpork(rpcClient, spork) {
  await rpcClient.sporkupdate(spork, 0);
}
