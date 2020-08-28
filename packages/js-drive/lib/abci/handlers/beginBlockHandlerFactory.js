const {
  abci: {
    ResponseBeginBlock,
  },
} = require('abci/types');

const NotSupportedProtocolVersionError = require('./errors/NotSupportedProtocolVersionError');

/**
 * Begin block ABCI handler
 *
 * @param {BlockchainState} blockchainState
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @param {BlockExecutionState} blockExecutionState
 * @param {Number} protocolVersion - Protocol version
 * @param {BaseLogger} logger
 *
 * @return {beginBlockHandler}
 */
function beginBlockHandlerFactory(
  blockchainState,
  blockExecutionDBTransactions,
  blockExecutionState,
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

    blockExecutionState.reset();

    blockExecutionState.setHeader(header);

    blockchainState.setLastBlockHeight(header.height);

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
