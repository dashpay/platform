const {
  abci: {
    ResponseDeliverTx,
  },
} = require('abci/types');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');

const InvalidArgumentAbciError = require('../errors/InvalidArgumentAbciError');

/**
 * @param {unserializeStateTransition} transactionalUnserializeStateTransition
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {BlockExecutionState} blockExecutionState
 *
 * @return {deliverTxHandler}
 */
function deliverTxHandlerFactory(
  transactionalUnserializeStateTransition,
  transactionalDpp,
  blockExecutionState,
) {
  /**
   * DeliverTx ABCI handler
   *
   * @typedef deliverTxHandler
   *
   * @param {abci.RequestDeliverTx} request
   * @return {Promise<abci.ResponseDeliverTx>}
   */
  async function deliverTxHandler({ tx: stateTransitionByteArray }) {
    const stateTransition = await transactionalUnserializeStateTransition(stateTransitionByteArray);

    const result = await transactionalDpp.stateTransition.validateData(stateTransition);

    if (!result.isValid()) {
      throw new InvalidArgumentAbciError('Invalid state transition', { errors: result.getErrors() });
    }

    await transactionalDpp.stateTransition.apply(stateTransition);

    // Save data contracts in order to create databases for documents on block commit
    if (stateTransition.getType() === stateTransitionTypes.DATA_CONTRACT_CREATE) {
      blockExecutionState.addDataContract(stateTransition.getDataContract());
    }

    // Reduce an identity balance and accumulate fees for all STs in the block
    // in order to store them in credits distribution pool
    const stateTransitionFee = stateTransition.calculateFee();

    const identity = await transactionalDpp.getStateRepository().fetchIdentity(
      stateTransition.getOwnerId(),
    );

    identity.reduceBalance(stateTransitionFee);

    await transactionalDpp.getStateRepository().storeIdentity(identity);

    blockExecutionState.incrementAccumulativeFees(stateTransitionFee);

    return new ResponseDeliverTx();
  }

  return deliverTxHandler;
}

module.exports = deliverTxHandlerFactory;
