const wait = require('../util/wait');

/**
 * Check that core is synced (factory)
 *
 * @param {RpcClient} coreRpcClient
 *
 * @returns {waitForCoreSync}
 */
function waitForCoreSyncFactory(coreRpcClient) {
  /**
   * Check that core is synced
   *
   * @typedef waitForCoreSync
   *
   * @param {function(number, number)} progressCallback
   *
   * @returns {Promise<void>}
   */
  async function waitForCoreSync(progressCallback) {
    let isBlockchainSynced = false;
    while (!isBlockchainSynced) {
      ({
        result: {
          IsBlockchainSynced: isBlockchainSynced,
        },
      } = await coreRpcClient.mnsync('status'));

      if (!isBlockchainSynced) {
        const {
          result: {
            blocks: currentBlockHeight,
            headers: currentHeadersNumber,
          },
        } = await coreRpcClient.getBlockchainInfo();

        progressCallback(currentBlockHeight, currentHeadersNumber);

        await wait(10000);
      }
    }
  }

  return waitForCoreSync;
}

module.exports = waitForCoreSyncFactory;
