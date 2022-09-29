const wait = require('../util/wait');

/**
 * Wait Core to be synced
 *
 * @typedef {waitForCoreSync}
 * @param {CoreService} coreService
 * @param {function(progress: {percent: number, blocks: number, headers: number})} [progressCallback]
 * @return {Promise<void>}
 */
async function waitForCoreSync(coreService, progressCallback = () => {}) {
  let isSynced = false;
  let isBlockchainSynced = false;
  let verificationProgress = 0.0;

  do {
    ({
      result: {IsSynced: isSynced, IsBlockchainSynced: isBlockchainSynced},
    } = await coreService.getRpcClient().mnsync('status'));

    ({
      result: {verificationprogress: verificationProgress, headers: headers, blocks: blocks},
    } = await coreService.getRpcClient().getBlockchainInfo());

    if (!isSynced || !isBlockchainSynced) {
      await wait(10000);
      progressCallback({percent: verificationProgress, headers, blocks});
    }
  } while (!isSynced || !isBlockchainSynced);
}

module.exports = waitForCoreSync;
