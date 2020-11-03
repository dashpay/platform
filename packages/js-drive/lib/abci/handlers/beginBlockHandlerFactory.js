const {
  abci: {
    ResponseBeginBlock,
  },
} = require('abci/types');

const NotSupportedProtocolVersionError = require('./errors/NotSupportedProtocolVersionError');

/**
 * Begin block ABCI handler
 *
 * @param {ChainInfo} chainInfo
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {Number} protocolVersion - Protocol version
 * @param {BaseLogger} logger
 *
 * @return {beginBlockHandler}
 */
function beginBlockHandlerFactory(
  chainInfo,
  blockExecutionDBTransactions,
  blockExecutionContext,
  protocolVersion,
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

    blockExecutionContext.reset();

    blockExecutionContext.setHeader(header);

    chainInfo.setLastBlockHeight(header.height);

    if (header.version.App.gt(protocolVersion)) {
      throw new NotSupportedProtocolVersionError(
        header.version.App,
        protocolVersion,
      );
    }

    await blockExecutionDBTransactions.start();

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
