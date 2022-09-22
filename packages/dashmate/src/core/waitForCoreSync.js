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
    try {
      const info = await coreService.dockerContainer.inspect()

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
    } catch (e) {
      if (info.State.Health.Status !== 'starting') {
        console.log(e)
        throw e
      }
    }
  } while (!isSynced || !isBlockchainSynced);
}

module.exports = waitForCoreSync;
