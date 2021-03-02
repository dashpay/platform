/**
 * @typedef activateCoreSpork
 * @param {RpcClient} rpcClient
 * @param {string} spork
 * @returns {Promise<void>}
 */
async function activateCoreSpork(rpcClient, spork) {
  await rpcClient.spork(spork, 0);
}

module.exports = activateCoreSpork;
