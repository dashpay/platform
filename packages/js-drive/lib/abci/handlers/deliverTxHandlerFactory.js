const {
  tendermint: {
    abci: {
      ResponseDeliverTx,
    },
  },
} = require('@dashevo/abci/types');

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
      .toString('hex')
      .toUpperCase();

    const consensusLogger = logger.child({
      height: blockHeight.toString(),
      txId: stHash,
      abciMethod: 'deliverTx',
    });

    blockExecutionContext.setConsensusLogger(consensusLogger);

    consensusLogger.info(`Deliver state transition ${stHash} from block #${blockHeight}`);

    let stateTransition;
    try {
      stateTransition = await transactionalUnserializeStateTransition(
        stateTransitionByteArray,
        {
          logger: consensusLogger,
        },
      );
    } catch (e) {
      blockExecutionContext.incrementInvalidTxCount();

      throw e;
    }

    const result = await transactionalDpp.stateTransition.validateData(stateTransition);

    if (!result.isValid()) {
      consensusLogger.info('State transition data is invalid');
      consensusLogger.debug({
        consensusErrors: result.getErrors(),
      });

      blockExecutionContext.incrementInvalidTxCount();

      throw new InvalidArgumentAbciError('Invalid state transition', { errors: result.getErrors() });
    }

    // Apply state transition to the state
    await transactionalDpp.stateTransition.apply(stateTransition);

    blockExecutionContext.incrementValidTxCount();

    // Reduce an identity balance and accumulate fees for all STs in the block
    // in order to store them in credits distribution pool
    const stateTransitionFee = stateTransition.calculateFee();

    const identity = await transactionalDpp.getStateRepository().fetchIdentity(
      stateTransition.getOwnerId(),
    );

    identity.reduceBalance(stateTransitionFee);

    await transactionalDpp.getStateRepository().storeIdentity(identity);

    blockExecutionContext.incrementAccumulativeFees(stateTransitionFee);

    // Logging
    switch (stateTransition.getType()) {
      case stateTransitionTypes.DATA_CONTRACT_CREATE: {
        const dataContract = stateTransition.getDataContract();

        // Save data contracts in order to create databases for documents on block commit
        blockExecutionContext.addDataContract(dataContract);

        consensusLogger.info(
          {
            dataContractId: dataContract.getId().toString(),
          },
          `Data contract created with id: ${dataContract.getId()}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_CREATE: {
        const identityId = stateTransition.getIdentityId();

        consensusLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Identity created with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_TOP_UP: {
        const identityId = stateTransition.getIdentityId();

        consensusLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Identity topped up with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.DOCUMENTS_BATCH: {
        stateTransition.getTransitions().forEach((transition) => {
          const description = DOCUMENT_ACTION_DESCRIPTONS[transition.getAction()];

          consensusLogger.info(
            {
              documentId: transition.getId().toString(),
            },
            `Document ${description} with id: ${transition.getId()}`,
          );
        });

        break;
      }
      default:
        break;
    }

    return new ResponseDeliverTx();
  }

  return deliverTxHandler;
}

module.exports = deliverTxHandlerFactory;
