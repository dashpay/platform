const wait = require('../util/wait');

/**
 * Wait Core to be synced
 *
 * @typedef {waitForCoreSync}
 * @param {CoreService} coreService
 * @return {Promise<void>}
 */
async function waitForCoreSync(coreService) {
  let isSynced = false;

  do {
    ({ result: { IsSynced: isSynced } } = await coreService.getRpcClient().mnsync('status'));

    if (!isSynced) {
      await wait(10000);
    }
  } while (!isSynced);
}

module.exports = waitForCoreSync;
