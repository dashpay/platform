const crypto = require('crypto');
const { FeeResult } = require('@dashevo/rs-drive');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const AbstractDocumentTransition = require(
  '@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition',
);

const DPPValidationAbciError = require('../../errors/DPPValidationAbciError');

const DOCUMENT_ACTION_DESCRIPTIONS = {
  [AbstractDocumentTransition.ACTIONS.CREATE]: 'Create',
  [AbstractDocumentTransition.ACTIONS.REPLACE]: 'Replace',
  [AbstractDocumentTransition.ACTIONS.DELETE]: 'Delete',
};

const DATA_CONTRACT_ACTION_DESCRIPTIONS = {
  [stateTransitionTypes.DATA_CONTRACT_CREATE]: 'Create',
  [stateTransitionTypes.DATA_CONTRACT_UPDATE]: 'Update',
};

const TIMERS = require('../timers');

/**
 * @param {unserializeStateTransition} transactionalUnserializeStateTransition
 * @param {DashPlatformProtocol} transactionalDpp
 * @param {BlockExecutionContext} proposalBlockExecutionContext
 * @param {ExecutionTimer} executionTimer
 * @param {IdentityBalanceStoreRepository} identityBalanceRepository
 * @param {calculateStateTransitionFee} calculateStateTransitionFee
 * @param {calculateStateTransitionFeeFromOperations} calculateStateTransitionFeeFromOperations
 * @param {createContextLogger} createContextLogger
 *
 * @return {deliverTx}
 */
function deliverTxFactory(
  transactionalUnserializeStateTransition,
  transactionalDpp,
  proposalBlockExecutionContext,
  executionTimer,
  identityBalanceRepository,
  calculateStateTransitionFee,
  calculateStateTransitionFeeFromOperations,
  createContextLogger,
) {
  /**
   * @typedef deliverTx
   *
   * @param {Buffer} stateTransitionByteArray
   * @param {number} round
   * @param {BaseLogger} contextLogger
   * @return {Promise<{
   *  code: number,
   *  fees: BlockFees}>}
   */
  async function deliverTx(stateTransitionByteArray, round, contextLogger) {
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

    const txContextLogger = createContextLogger(contextLogger, {
      txId: stHash,
    });

    txContextLogger.info(`Deliver state transition ${stHash} from block #${blockHeight}`);

    const stateTransition = await transactionalUnserializeStateTransition(
      stateTransitionByteArray,
      {
        logger: txContextLogger,
        executionTimer,
      },
    );

    // Logging
    /* istanbul ignore next */
    switch (stateTransition.getType()) {
      case stateTransitionTypes.DATA_CONTRACT_UPDATE:
      case stateTransitionTypes.DATA_CONTRACT_CREATE: {
        const dataContract = stateTransition.getDataContract();

        // Save data contracts in order to create databases for documents on block commit
        proposalBlockExecutionContext.addDataContract(dataContract);

        const description = DATA_CONTRACT_ACTION_DESCRIPTIONS[stateTransition.getType()];

        txContextLogger.info(
          {
            dataContractId: dataContract.getId().toString(),
          },
          `${description} Data Contract with id: ${dataContract.getId()}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_CREATE: {
        const identityId = stateTransition.getIdentityId();

        txContextLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Create Identity with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_TOP_UP: {
        const identityId = stateTransition.getIdentityId();

        txContextLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Top up Identity with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_UPDATE: {
        const identityId = stateTransition.getIdentityId();

        txContextLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Update Identity with id: ${identityId}`,
        );
        break;
      }
      case stateTransitionTypes.DOCUMENTS_BATCH: {
        stateTransition.getTransitions().forEach((transition) => {
          const description = DOCUMENT_ACTION_DESCRIPTIONS[transition.getAction()];

          txContextLogger.info(
            {
              documentId: transition.getId().toString(),
            },
            `${description} Document with id: ${transition.getId()}`,
          );
        });

        break;
      }
      default:
        break;
    }

    // Remove dry run operations from state transition execution context

    const stateTransitionExecutionContext = stateTransition.getExecutionContext();

    const predictedStateTransitionOperations = stateTransitionExecutionContext.getOperations();
    const predictedStateTransitionFees = calculateStateTransitionFeeFromOperations(
      predictedStateTransitionOperations,
      stateTransition.getOwnerId(),
    );

    stateTransitionExecutionContext.clearDryOperations();

    // Validate against state

    executionTimer.startTimer(TIMERS.DELIVER_TX.VALIDATE_STATE);

    const result = await transactionalDpp.stateTransition.validateState(stateTransition);

    if (!result.isValid()) {
      const consensusError = result.getFirstError();
      const message = 'State transition is invalid against the state';

      txContextLogger.info(message);
      txContextLogger.debug({
        consensusError,
      });

      throw new DPPValidationAbciError(message, result.getFirstError());
    }

    executionTimer.stopTimer(TIMERS.DELIVER_TX.VALIDATE_STATE, true);

    executionTimer.startTimer(TIMERS.DELIVER_TX.APPLY);

    // Apply state transition to the state

    await transactionalDpp.stateTransition.apply(stateTransition);

    executionTimer.stopTimer(TIMERS.DELIVER_TX.APPLY, true);

    // Update identity balance

    const actualStateTransitionFees = calculateStateTransitionFee(stateTransition);

    const actualStateTransitionOperations = stateTransition.getExecutionContext().getOperations();

    if (actualStateTransitionFees.desiredAmount > predictedStateTransitionFees.desiredAmount) {
      txContextLogger.warn({
        predictedFee: predictedStateTransitionFees.desiredAmount,
        actualFee: actualStateTransitionFees.desiredAmount,
      }, `Actual fees are greater than predicted for ${actualStateTransitionFees.desiredAmount - predictedStateTransitionFees.desiredAmount} credits`);
    }

    const feeResult = FeeResult.create(
      actualStateTransitionFees.storageFee,
      actualStateTransitionFees.processingFee,
      actualStateTransitionFees.feeRefunds,
    );

    const applyFeesToBalanceResult = await identityBalanceRepository.applyFees(
      stateTransition.getOwnerId(),
      feeResult,
      { useTransaction: true },
    );

    const transactionFees = applyFeesToBalanceResult.getValue();

    const deliverTxTiming = executionTimer.stopTimer(TIMERS.DELIVER_TX.OVERALL);

    txContextLogger.trace(
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
            storage: predictedStateTransitionFees.storageFee,
            processing: predictedStateTransitionFees.processingFee,
            refunds: predictedStateTransitionFees.totalRefunds,
            requiredAmount: predictedStateTransitionFees.requiredAmount,
            desiredAmount: predictedStateTransitionFees.desiredAmount,
            operations: predictedStateTransitionOperations.map((operation) => operation.toJSON()),
          },
          actual: {
            storage: actualStateTransitionFees.storageFee,
            processing: actualStateTransitionFees.processingFee,
            refunds: actualStateTransitionFees.totalRefunds,
            requiredAmount: actualStateTransitionFees.requiredAmount,
            desiredAmount: actualStateTransitionFees.desiredAmount,
            operations: actualStateTransitionOperations.map((operation) => operation.toJSON()),
          },
          debt: actualStateTransitionFees.desiredAmount - transactionFees.processingFee,
        },
        txType: stateTransition.getType(),
      },
      `${stateTransition.constructor.name} execution took ${deliverTxTiming} seconds and cost ${actualStateTransitionFees.desiredAmount} credits`,
    );

    return {
      code: 0,
      fees: {
        storageFee: transactionFees.storageFee,
        processingFee: transactionFees.processingFee,
        refundsPerEpoch: transactionFees.sumFeeRefundsPerEpoch(),
      },
    };
  }

  return deliverTx;
}

module.exports = deliverTxFactory;
