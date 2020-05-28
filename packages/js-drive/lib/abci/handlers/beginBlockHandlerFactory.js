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
  async function beginBlockHandler({ header: { height } }) {
    blockchainState.setLastBlockHeight(height);

    await blockExecutionDBTransactions.start();

    blockExecutionState.reset();

    return new ResponseBeginBlock();
  }

  return beginBlockHandler;
}

module.exports = beginBlockHandlerFactory;
