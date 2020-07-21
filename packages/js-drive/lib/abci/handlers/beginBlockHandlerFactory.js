const {
  abci: {
    ResponseBeginBlock,
  },
} = require('abci/types');

/**
 * Begin block ABCI handler
 *
 * @param {BlockchainState} blockchainState
 * @param {BlockExecutionDBTransactions} blockExecutionDBTransactions
 * @param {BlockExecutionState} blockExecutionState
 * @param {BaseLogger} logger
 *
 * @return {beginBlockHandler}
 */
function beginBlockHandlerFactory(
  blockchainState,
  blockExecutionDBTransactions,
  blockExecutionState,
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

    await blockExecutionDBTransactions.start();

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
