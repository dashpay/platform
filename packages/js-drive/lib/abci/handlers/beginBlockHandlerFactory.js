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
 *
 * @return {beginBlockHandler}
 */
function beginBlockHandlerFactory(
  blockchainState,
  blockExecutionDBTransactions,
  blockExecutionState,
) {
  /**
   * @typedef beginBlockHandler
   *
   * @param {abci.RequestBeginBlock} request
   * @return {Promise<abci.ResponseBeginBlock>}
   */
  async function beginBlockHandler({ header }) {
    blockExecutionState.reset();

    blockExecutionState.setHeader(header);

    blockchainState.setLastBlockHeight(header.height);

    await blockExecutionDBTransactions.start();

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
