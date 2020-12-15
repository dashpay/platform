const {
  tendermint: {
    abci: {
      ResponseBeginBlock,
    },
  },
} = require('@dashevo/abci/types');

const NotSupportedProtocolVersionError = require('./errors/NotSupportedProtocolVersionError');

/**
 * Begin block ABCI handler
 *
 * @param {ChainInfo} chainInfo
 * @param {BlockExecutionStoreTransactions} blockExecutionStoreTransactions
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {Number} protocolVersion - Protocol version
 * @param {updateSimplifiedMasternodeList} updateSimplifiedMasternodeList
 * @param {waitForChainlockedHeight} waitForChainlockedHeight
 * @param {BaseLogger} logger
 *
 * @return {beginBlockHandler}
 */
function beginBlockHandlerFactory(
  chainInfo,
  blockExecutionStoreTransactions,
  blockExecutionContext,
  protocolVersion,
  updateSimplifiedMasternodeList,
  waitForChainlockedHeight,
  logger,
) {
  /**
   * @typedef beginBlockHandler
   *
   * @param {abci.RequestBeginBlock} request
   * @return {Promise<abci.ResponseBeginBlock>}
   */
  async function beginBlockHandler({ header }) {
    logger.info(`Block begin #${header.height}`);

    const { coreChainLockedHeight } = header;

    await waitForChainlockedHeight(coreChainLockedHeight);

    await updateSimplifiedMasternodeList(coreChainLockedHeight);

    blockExecutionContext.reset();

    blockExecutionContext.setHeader(header);

    chainInfo.setLastBlockHeight(header.height);

    if (header.version.app.gt(protocolVersion)) {
      throw new NotSupportedProtocolVersionError(
        header.version.app,
        protocolVersion,
      );
    }

    await blockExecutionStoreTransactions.start();

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
