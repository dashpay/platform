const wait = require('../util/wait');

/**
 * Wait Core to be synced
 *
 * @typedef {waitForCoreSync}
 * @param {RpcClient} rpcClient
 * @param {function(progress: {percent: number, blocks: number, headers: number})} [progressCallback]
 * @return {Promise<void>}
 */
async function waitForCoreSync(rpcClient, progressCallback = () => {}) {
  let isSynced = false;
  let isBlockchainSynced = false;
  let verificationProgress = 0.0;

  do {
    try {
      ({
        result: {IsSynced: isSynced, IsBlockchainSynced: isBlockchainSynced},
      } = await rpcClient.mnsync('status'));

      ({
        result: {verificationprogress: verificationProgress, headers: headers, blocks: blocks},
      } = await rpcClient.getBlockchainInfo());

      if (!isSynced || !isBlockchainSynced) {
        await wait(10000);
        progressCallback({percent: verificationProgress, headers, blocks});
      }
    } catch (e) {
      console.log(e)
    }
  } while (!isSynced || !isBlockchainSynced);
}

module.exports = waitForCoreSync;
