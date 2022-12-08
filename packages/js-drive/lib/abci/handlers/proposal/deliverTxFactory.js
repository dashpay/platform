const crypto = require('crypto');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const AbstractDocumentTransition = require(
  '@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition',
);

const calculateOperationFees = require('@dashevo/dpp/lib/stateTransition/fee/calculateOperationFees');
const aggregateOperationFees = require('./fees/aggregateOperationFees');

const DPPValidationAbciError = require('../../errors/DPPValidationAbciError');

const DOCUMENT_ACTION_DESCRIPTIONS = {
  [AbstractDocumentTransition.ACTIONS.CREATE]: 'created',
  [AbstractDocumentTransition.ACTIONS.REPLACE]: 'replaced',
  [AbstractDocumentTransition.ACTIONS.DELETE]: 'deleted',
};

const DATA_CONTRACT_ACTION_DESCRIPTIONS = {
  [stateTransitionTypes.DATA_CONTRACT_CREATE]: 'created',
  [stateTransitionTypes.DATA_CONTRACT_UPDATE]: 'updated',
};

const TIMERS = require('../timers');

/**
 * @param {unserializeStateTransition} transactionalUnserializeStateTransition
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {ExecutionTimer} executionTimer
 *
 * @return {deliverTx}
 */
function deliverTxFactory(
  transactionalUnserializeStateTransition,
  transactionalDpp,
  proposalBlockExecutionContext,
  executionTimer,
) {
  /**
   * @typedef deliverTx
   *
   * @param {Buffer} stateTransitionByteArray
   * @param {number} round
   * @param {BaseLogger} consensusLogger
   * @return {Promise<{ code: number, fees: FeeResult }>}
   */
  async function deliverTx(stateTransitionByteArray, round, consensusLogger) {
    const blockHeight = proposalBlockExecutionContext.getHeight();

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

    const txConsensusLogger = consensusLogger.child({
      txId: stHash,
    });

    proposalBlockExecutionContext.setConsensusLogger(txConsensusLogger);

    txConsensusLogger.info(`Deliver state transition ${stHash} from block #${blockHeight}`);

    const stateTransition = await transactionalUnserializeStateTransition(
      stateTransitionByteArray,
      {
        logger: txConsensusLogger,
        executionTimer,
      },
    );

    // Keep only actual operations
    const stateTransitionExecutionContext = stateTransition.getExecutionContext();

    const predictedStateTransitionFee = stateTransition.calculateFee();
    const predictedStateTransitionOperations = stateTransitionExecutionContext.getOperations();

    stateTransitionExecutionContext.clearDryOperations();

    executionTimer.startTimer(TIMERS.DELIVER_TX.VALIDATE_STATE);

    const result = await transactionalDpp.stateTransition.validateState(stateTransition);

    if (!result.isValid()) {
      const consensusError = result.getFirstError();
      const message = 'State transition is invalid against the state';

      txConsensusLogger.info(message);
      txConsensusLogger.debug({
        consensusError,
      });

      throw new DPPValidationAbciError(message, result.getFirstError());
    }

    executionTimer.stopTimer(TIMERS.DELIVER_TX.VALIDATE_STATE, true);

    executionTimer.startTimer(TIMERS.DELIVER_TX.APPLY);

    // Apply state transition to the state
    await transactionalDpp.stateTransition.apply(stateTransition);

    executionTimer.stopTimer(TIMERS.DELIVER_TX.APPLY, true);

    // Reduce an identity balance and accumulate fees for all STs in the block
    // in order to store them in credits distribution pool
    const actualStateTransitionFee = stateTransition.calculateFee();

    // TODO: enable once fee calculation is done
    // if (actualStateTransitionFee > predictedStateTransitionFee) {
    //   throw new PredictedFeeLowerThanActualError(
    //     predictedStateTransitionFee,
    //     actualStateTransitionFee,
    //     stateTransition,
    //   );
    // }

    const identity = await transactionalDpp.getStateRepository().fetchIdentity(
      stateTransition.getOwnerId(),
    );

    // TODO: We should increment identity balance debt in case if it goes negative
    let updatedBalance = identity.getBalance() - actualStateTransitionFee;

    if (updatedBalance < 0) {
      updatedBalance = 0;
    }

    identity.setBalance(updatedBalance);

    await transactionalDpp.getStateRepository().updateIdentity(identity);

    // Logging
    /* istanbul ignore next */
    switch (stateTransition.getType()) {
      case stateTransitionTypes.DATA_CONTRACT_UPDATE:
      case stateTransitionTypes.DATA_CONTRACT_CREATE: {
        const dataContract = stateTransition.getDataContract();

        // Save data contracts in order to create databases for documents on block commit
        proposalBlockExecutionContext.addDataContract(dataContract);

        const description = DATA_CONTRACT_ACTION_DESCRIPTIONS[stateTransition.getType()];

        txConsensusLogger.info(
          {
            dataContractId: dataContract.getId().toString(),
          },
          `Data contract ${description} with id: ${dataContract.getId()}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_CREATE: {
        const identityId = stateTransition.getIdentityId();

        txConsensusLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Identity created with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_TOP_UP: {
        const identityId = stateTransition.getIdentityId();

        txConsensusLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Identity topped up with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_UPDATE: {
        const identityId = stateTransition.getIdentityId();

        txConsensusLogger.info(
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

          txConsensusLogger.info(
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

    const actualStateTransitionOperations = stateTransition.getExecutionContext().getOperations();

    const {
      storageFee: actualStorageFees,
      processingFee: actualProcessingFees,
    } = calculateOperationFees(actualStateTransitionOperations);

    const {
      storageFee: predictedStorageFee,
      processingFee: predictedProcessingFee,
    } = calculateOperationFees(predictedStateTransitionOperations);

    txConsensusLogger.trace(
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
          predicted: {
            storage: predictedStorageFee,
            processing: predictedProcessingFee,
            final: predictedStateTransitionFee,
            operations: predictedStateTransitionOperations.map((operation) => operation.toJSON()),
          },
          actual: {
            storage: actualStorageFees,
            processing: actualProcessingFees,
            final: actualStateTransitionFee,
            operations: actualStateTransitionOperations.map((operation) => operation.toJSON()),
          },
        },
        txType: stateTransition.getType(),
      },
      `${stateTransition.constructor.name} execution took ${deliverTxTiming} seconds and cost ${actualStateTransitionFee} credits`,
    );

    return {
      code: 0,
      fees: aggregateOperationFees(actualStateTransitionOperations),
    };
  }

  return deliverTx;
}

module.exports = deliverTxFactory;
