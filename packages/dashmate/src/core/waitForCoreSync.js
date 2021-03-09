const wait = require('../util/wait');

/**
 * Wait Core to be synced
 *
 * @typedef {waitForCoreSync}
 * @param {CoreService} coreService
 * @param {function(progress: number)} progressCallback
 * @return {Promise<void>}
 */
async function waitForCoreSync(coreService, progressCallback) {
  let isSynced = false;
  let verificationProgress = 0.0;

  do {
    ({
      result: { IsSynced: isSynced },
    } = await coreService.getRpcClient().mnsync('status'));
    ({
      result: { verificationprogress: verificationProgress },
    } = await coreService.getRpcClient().getBlockchainInfo());

    if (!isSynced) {
      await wait(10000);
      progressCallback(verificationProgress);
    }
  } while (!isSynced);
}

module.exports = waitForCoreSync;
