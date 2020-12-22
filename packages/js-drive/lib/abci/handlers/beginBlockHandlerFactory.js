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
 * @param {waitForChainLockedHeight} waitForChainLockedHeight
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
  waitForChainLockedHeight,
  logger,
) {
  /**
   * @typedef beginBlockHandler
   *
   * @param {abci.RequestBeginBlock} request
   * @return {Promise<abci.ResponseBeginBlock>}
   */
  async function beginBlockHandler({ header }) {
    const {
      coreChainLockedHeight,
      height,
      version,
    } = header;

    logger.info(`Block begin #${height}`);

    await waitForChainLockedHeight(coreChainLockedHeight);

    await updateSimplifiedMasternodeList(coreChainLockedHeight);

    // in case previous block execution failed in process
    // and not commited. We need to make sure
    // previous context is reset.
    blockExecutionContext.reset();

    blockExecutionContext.setHeader(header);

    chainInfo.setLastBlockHeight(height);
    chainInfo.setLastCoreChainLockedHeight(coreChainLockedHeight);

    if (version.app.gt(protocolVersion)) {
      throw new NotSupportedProtocolVersionError(
        version.app,
        protocolVersion,
      );
    }

    if (blockExecutionStoreTransactions.isStarted()) {
      // in case previous block execution failed in process
      // and not commited. We need to make sure
      // previous transactions are aborted.
      await blockExecutionStoreTransactions.abort();
    }

    await blockExecutionStoreTransactions.start();

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
