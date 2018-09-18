const ChainInfo = require('./ChainInfo');

/**
 * @param {RpcClient} rpcClient
 * @returns {getChainInfo}
 */
function getChainInfoFactory(rpcClient) {
  /**
   * @typedef getChainInfo
   * @returns {Promise<ChainInfo>}
   */
  async function getChainInfo() {
    const [chainInfo, mnSyncStatus] = await Promise.all([
      rpcClient.getBlockchainInfo(),
      rpcClient.mnsync('status'),
    ]);
    const {
      result: {
        blocks: lastChainBlockHeight,
        bestblockhash: lastChainBlockHash,
      },
    } = chainInfo;
    const { result: { IsBlockchainSynced } } = mnSyncStatus;
    return new ChainInfo(
      lastChainBlockHeight,
      lastChainBlockHash,
      IsBlockchainSynced,
    );
  }

  return getChainInfo;
}

module.exports = getChainInfoFactory;
