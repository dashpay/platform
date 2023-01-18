const crypto = require('crypto');
const { FeeResult } = require('@dashevo/rs-drive');

const stateTransitionTypes = require('@dashevo/dpp/lib/stateTransition/stateTransitionTypes');
const AbstractDocumentTransition = require(
  '@dashevo/dpp/lib/document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition',
);

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
 * @param {IdentityStoreRepository} identityRepository
 *
 * @return {deliverTx}
 */
function deliverTxFactory(
  transactionalUnserializeStateTransition,
  transactionalDpp,
  proposalBlockExecutionContext,
  executionTimer,
  identityRepository,
) {
  /**
   * @typedef deliverTx
   *
   * @param {Buffer} stateTransitionByteArray
   * @param {number} round
   * @param {BaseLogger} consensusLogger
   * @return {Promise<{
   *  code: number,
   *  fees: BlockFees}>}
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

    // Remove dry run operations from state transition execution context

    const stateTransitionExecutionContext = stateTransition.getExecutionContext();

    const predictedStateTransitionOperations = stateTransitionExecutionContext.getOperations();
    const predictedStateTransitionFees = stateTransitionExecutionContext
      .getLastCalculatedFeeDetails();

    stateTransitionExecutionContext.clearDryOperations();

    // Validate against state

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

    // Update identity balance

    const actualStateTransitionFees = stateTransitionExecutionContext
      .getLastCalculatedFeeDetails();
    const actualStateTransitionOperations = stateTransition.getExecutionContext().getOperations();

    if (actualStateTransitionFees.desiredAmount > predictedStateTransitionFees.desiredAmount) {
      txConsensusLogger.warn({
        predictedFee: predictedStateTransitionFees.desiredAmount,
        actualFee: actualStateTransitionFees.desiredAmount,
      }, `Actual fees are greater than predicted for ${actualStateTransitionFees.desiredAmount - predictedStateTransitionFees.desiredAmount} credits`);
    }

    const feeResult = FeeResult.create(
      actualStateTransitionFees.storageFee,
      actualStateTransitionFees.processingFee,
      actualStateTransitionFees.feeRefunds,
    );

    const applyFeesToBalanceResult = await identityRepository.applyFeesToBalance(
      stateTransition.getOwnerId(),
      feeResult,
      { useTransaction: true },
    );

    const transactionFees = applyFeesToBalanceResult.getValue();

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
