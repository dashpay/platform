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
  '@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition',
);

const calculateOperationCosts = require('@dashevo/dpp/lib/stateTransition/fee/calculateOperationCosts');

const DPPValidationAbciError = require('../errors/DPPValidationAbciError');

const DOCUMENT_ACTION_DESCRIPTIONS = {
  [AbstractDocumentTransition.ACTIONS.CREATE]: 'created',
  [AbstractDocumentTransition.ACTIONS.REPLACE]: 'replaced',
  [AbstractDocumentTransition.ACTIONS.DELETE]: 'deleted',
};

const DATA_CONTRACT_ACTION_DESCRIPTIONS = {
  [stateTransitionTypes.DATA_CONTRACT_CREATE]: 'created',
  [stateTransitionTypes.DATA_CONTRACT_UPDATE]: 'updated',
};

const TIMERS = require('./timers');

/**
 * @param {unserializeStateTransition} transactionalUnserializeStateTransition
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {BlockExecutionContext} blockExecutionContext
 * @param {BaseLogger} logger
 * @param {ExecutionTimer} executionTimer
 *
 * @return {deliverTxHandler}
 */
function deliverTxHandlerFactory(
  transactionalUnserializeStateTransition,
  transactionalDpp,
  blockExecutionContext,
  logger,
  executionTimer,
) {
  /**
   * DeliverTx ABCI Handler
   *
   * @typedef deliverTxHandler
   *
   * @param {abci.RequestDeliverTx} request
   * @return {Promise<abci.ResponseDeliverTx>}
   */
  async function deliverTxHandler({ tx: stateTransitionByteArray }) {
    const { height: blockHeight } = blockExecutionContext.getHeader();

    // Start execution timer

    executionTimer.clearTimer(TIMERS.DELIVER_TX.OVERALL);
    executionTimer.clearTimer(TIMERS.DELIVER_TX.VALIDATE_BASIC);
    executionTimer.clearTimer(TIMERS.DELIVER_TX.VALIDATE_FEE);
    executionTimer.clearTimer(TIMERS.DELIVER_TX.VALIDATE_SIGNATURE);
    executionTimer.clearTimer(TIMERS.DELIVER_TX.VALIDATE_STATE);
    executionTimer.clearTimer(TIMERS.DELIVER_TX.APPLY);

    executionTimer.startTimer(TIMERS.DELIVER_TX.OVERALL);

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
          executionTimer,
        },
      );
    } catch (e) {
      blockExecutionContext.incrementInvalidTxCount();

      throw e;
    }

    executionTimer.startTimer(TIMERS.DELIVER_TX.VALIDATE_STATE);

    const result = await transactionalDpp.stateTransition.validateState(stateTransition);

    if (!result.isValid()) {
      const consensusError = result.getFirstError();
      const message = 'State transition is invalid against the state';

      consensusLogger.info(message);
      consensusLogger.debug({
        consensusError,
      });

      blockExecutionContext.incrementInvalidTxCount();

      throw new DPPValidationAbciError(message, result.getFirstError());
    }

    executionTimer.stopTimer(TIMERS.DELIVER_TX.VALIDATE_STATE, true);

    executionTimer.startTimer(TIMERS.DELIVER_TX.APPLY);

    // Apply state transition to the state
    await transactionalDpp.stateTransition.apply(stateTransition);

    executionTimer.stopTimer(TIMERS.DELIVER_TX.APPLY, true);

    blockExecutionContext.incrementValidTxCount();

    // Reduce an identity balance and accumulate fees for all STs in the block
    // in order to store them in credits distribution pool
    const stateTransitionFee = stateTransition.calculateFee();

    const identity = await transactionalDpp.getStateRepository().fetchIdentity(
      stateTransition.getOwnerId(),
    );

    // TODO: Temporary disabled until we calculate fee for validate state and apply functions
    // const updatedBalance = identity.reduceBalance(stateTransitionFee);

    // if (updatedBalance <= 0) {
    //   throw new NegativeBalanceError(identity);
    // }

    await transactionalDpp.getStateRepository().storeIdentity(identity);

    blockExecutionContext.incrementCumulativeFees(stateTransitionFee);

    // Logging
    switch (stateTransition.getType()) {
      case stateTransitionTypes.DATA_CONTRACT_UPDATE:
      case stateTransitionTypes.DATA_CONTRACT_CREATE: {
        const dataContract = stateTransition.getDataContract();

        // Save data contracts in order to create databases for documents on block commit
        blockExecutionContext.addDataContract(dataContract);

        const description = DATA_CONTRACT_ACTION_DESCRIPTIONS[stateTransition.getType()];

        consensusLogger.info(
          {
            dataContractId: dataContract.getId().toString(),
          },
          `Data contract ${description} with id: ${dataContract.getId()}`,
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
      case stateTransitionTypes.IDENTITY_UPDATE: {
        const identityId = stateTransition.getIdentityId();

        consensusLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Identity updated with id: ${identityId}`,
        );
        break;
      }
      case stateTransitionTypes.DOCUMENTS_BATCH: {
        stateTransition.getTransitions().forEach((transition) => {
          const description = DOCUMENT_ACTION_DESCRIPTIONS[transition.getAction()];

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

    const deliverTxTiming = executionTimer.stopTimer(TIMERS.DELIVER_TX.OVERALL);

    const stateTransitionOperations = stateTransition.getExecutionContext().getOperations();

    const {
      storageCost,
      processingCost,
    } = calculateOperationCosts(stateTransitionOperations);

    consensusLogger.trace(
      {
        timings: {
          overall: deliverTxTiming,
          validateBasic: executionTimer.getTimer(TIMERS.DELIVER_TX.VALIDATE_BASIC, true),
          validateFee: executionTimer.getTimer(TIMERS.DELIVER_TX.VALIDATE_FEE, true),
          validateSignature: executionTimer.getTimer(TIMERS.DELIVER_TX.VALIDATE_SIGNATURE, true),
          validateState: executionTimer.getTimer(TIMERS.DELIVER_TX.VALIDATE_STATE, true),
          apply: executionTimer.getTimer(TIMERS.DELIVER_TX.APPLY, true),
        },
        fees: {
          storage: storageCost,
          processing: processingCost,
          final: stateTransitionFee,
          operations: stateTransitionOperations.map((operation) => operation.toJSON()),
        },
        txType: stateTransition.getType(),
      },
      `${stateTransition.constructor.name} execution took ${deliverTxTiming} seconds and cost ${stateTransitionFee} credits`,
    );

    return new ResponseDeliverTx();
  }

  return deliverTxHandler;
}

module.exports = deliverTxHandlerFactory;
