const wait = require('../util/wait');

/**
 * Check is core in regtest mode with no peers
 *
 * @param {RpcClient} coreRpcClient
 *
 * @returns {waitForCoreSync}
 */
function detectStandaloneRegtestModeFactory(coreRpcClient) {
  /**
   * @typedef detectStandaloneRegtestMode
   *
   * @return {Promise<boolean>}
   */
  async function detectStandaloneRegtestMode() {
    const { result: blockchainInfo } = await coreRpcClient.getBlockchainInfo();
    if (blockchainInfo.chain === 'regtest') {
      // wait for core to connect to peers
      await wait(5000);

      const { result: peerInfo } = await coreRpcClient.getPeerInfo();
      if (peerInfo.length === 0) {
        return true;
      }
    }

    return false;
  }

  return detectStandaloneRegtestMode;
}

module.exports = detectStandaloneRegtestModeFactory;
