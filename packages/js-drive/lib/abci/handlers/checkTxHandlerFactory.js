const {
  tendermint: {
    abci: {
      ResponseCheckTx,
    },
  },
} = require('@dashevo/abci/types');

/**
 * @param {unserializeStateTransition} unserializeStateTransition
 * @param {AsyncLocalStorage} unserializeStateTransition
 * @param {createContextLogger} createContextLogger
 * @param {Logger} logger
 *
 * @returns {checkTxHandler}
 */
function checkTxHandlerFactory(
  unserializeStateTransition,
  createContextLogger,
  logger,
) {
  /**
   * CheckTx ABCI Handler
   *
   * @typedef checkTxHandler
   *
   * @param {abci.RequestCheckTx} request
   *
   * @returns {Promise<abci.ResponseCheckTx>}
   */
  async function checkTxHandler({ tx: stateTransitionByteArray }) {
    createContextLogger(logger, {
      abciMethod: 'checkTx',
    });

    await unserializeStateTransition(stateTransitionByteArray);

    return new ResponseCheckTx();
  }

  return checkTxHandler;
}

module.exports = checkTxHandlerFactory;
