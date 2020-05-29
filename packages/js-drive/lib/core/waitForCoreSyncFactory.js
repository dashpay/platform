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
    const { result: blockchainInfo } = await coreRpcClient.getBlockchainInfo();

    if (blockchainInfo.chain === 'regtest') {
      // wait for core to connect to peers
      await wait(5000);

      const { result: peerInfo } = await coreRpcClient.getPeerInfo();
      if (peerInfo.length === 0) {
        return;
      }
    }

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
