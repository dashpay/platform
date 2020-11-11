const {
  abci: {
    ResponseDeliverTx,
  },
} = require('abci/types');

const crypto = require('crypto');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const AbstractDocumentTransition = require(
  '@dashevo/dpp/lib/document/stateTransition/documentTransition/AbstractDocumentTransition',
);

const InvalidArgumentAbciError = require('../errors/InvalidArgumentAbciError');

const DOCUMENT_ACTION_DESCRIPTONS = {
  [AbstractDocumentTransition.ACTIONS.CREATE]: 'created',
  [AbstractDocumentTransition.ACTIONS.REPLACE]: 'replaced',
  [AbstractDocumentTransition.ACTIONS.DELETE]: 'deleted',
};

/**
 * @param {unserializeStateTransition} transactionalUnserializeStateTransition
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BaseLogger} logger
 *
 * @return {deliverTxHandler}
 */
function deliverTxHandlerFactory(
  transactionalUnserializeStateTransition,
  transactionalDpp,
  blockExecutionContext,
  logger,
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
    const { height: blockHeight } = blockExecutionContext.getHeader();

    const stHash = crypto
      .createHash('sha256')
      .update(stateTransitionByteArray)
      .digest()
      .toString('hex');

    logger.info(`Deliver state transition ${stHash} from block #${blockHeight}`);

    let stateTransition;
    try {
      stateTransition = await transactionalUnserializeStateTransition(stateTransitionByteArray);
    } catch (e) {
      if (e instanceof InvalidArgumentAbciError) {
        logger.info('State transition structure is invalid');
        logger.debug({
          error: e,
        });
      }

      throw e;
    }

    const result = await transactionalDpp.stateTransition.validateData(stateTransition);

    if (!result.isValid()) {
      logger.info('State transition data is invalid');
      logger.debug({
        errors: result.getErrors(),
      });
      throw new InvalidArgumentAbciError('Invalid state transition', { errors: result.getErrors() });
    }

    await transactionalDpp.stateTransition.apply(stateTransition);

    switch (stateTransition.getType()) {
      case stateTransitionTypes.DATA_CONTRACT_CREATE: {
        const dataContract = stateTransition.getDataContract();

        // Save data contracts in order to create databases for documents on block commit
        blockExecutionContext.addDataContract(dataContract);

        logger.info(`Data contract created with id: ${dataContract.getId()}`);

        break;
      }
      case stateTransitionTypes.IDENTITY_CREATE: {
        const identityId = stateTransition.getIdentityId();
        logger.info(`Identity created with id: ${identityId}`);
        break;
      }
      case stateTransitionTypes.DOCUMENTS_BATCH: {
        stateTransition.getTransitions().forEach((transition) => {
          const description = DOCUMENT_ACTION_DESCRIPTONS[transition.getAction()];
          logger.info(`Document ${description} with id: ${transition.getId()}`);
        });
        break;
      }
      default:
        break;
    }

    // Reduce an identity balance and accumulate fees for all STs in the block
    // in order to store them in credits distribution pool
    const stateTransitionFee = stateTransition.calculateFee();

    const identity = await transactionalDpp.getStateRepository().fetchIdentity(
      stateTransition.getOwnerId(),
    );

    identity.reduceBalance(stateTransitionFee);

    await transactionalDpp.getStateRepository().storeIdentity(identity);

    blockExecutionContext.incrementAccumulativeFees(stateTransitionFee);

    return new ResponseDeliverTx();
  }

  return deliverTxHandler;
}

module.exports = deliverTxHandlerFactory;
