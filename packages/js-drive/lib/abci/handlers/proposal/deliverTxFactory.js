const crypto = require('crypto');

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
 * @param {createContextLogger} createContextLogger
 *
 * @return {deliverTx}
 */
function deliverTxFactory(
  transactionalUnserializeStateTransition,
  transactionalDpp,
  proposalBlockExecutionContext,
  executionTimer,
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
   *  fees: BlockFeeResult}>}
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

    // Keep only actual operations
    const stateTransitionExecutionContext = stateTransition.getExecutionContext();

    const predictedStateTransitionOperations = stateTransitionExecutionContext.getOperations();
    const predictedStateTransitionFees = stateTransitionExecutionContext
      .getLastCalculatedFeeDetails();

    stateTransitionExecutionContext.clearDryOperations();

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

    // Reduce an identity balance and accumulate fees for all STs in the block
    // in order to store them in credits distribution pool
    const actualStateTransitionFees = stateTransitionExecutionContext
      .getLastCalculatedFeeDetails();
    const actualStateTransitionOperations = stateTransition.getExecutionContext().getOperations();

    if (actualStateTransitionFees.total > predictedStateTransitionFees.total) {
      txContextLogger.warn({
        predictedFee: predictedStateTransitionFees.total,
        actualFee: actualStateTransitionFees.total,
      }, `Actual fees are greater than predicted for ${actualStateTransitionFees.total - predictedStateTransitionFees.total} credits`);
    }

    const identity = await transactionalDpp.getStateRepository().fetchIdentity(
      stateTransition.getOwnerId(),
    );

    let updatedBalance = identity.getBalance() - actualStateTransitionFees.total;

    // TODO: We should increment identity balance debt in case if it goes negative
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

        txContextLogger.info(
          {
            dataContractId: dataContract.getId().toString(),
          },
          `Data contract ${description} with id: ${dataContract.getId()}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_CREATE: {
        const identityId = stateTransition.getIdentityId();

        txContextLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Identity created with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_TOP_UP: {
        const identityId = stateTransition.getIdentityId();

        txContextLogger.info(
          {
            identityId: identityId.toString(),
          },
          `Identity topped up with id: ${identityId}`,
        );

        break;
      }
      case stateTransitionTypes.IDENTITY_UPDATE: {
        const identityId = stateTransition.getIdentityId();

        txContextLogger.info(
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
          const dataContract = transition.getDataContract();

          txContextLogger.info(
            {
              documentId: transition.getId().toString(),
              dataContractId: dataContract.getId().toString(),
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
            refunds: predictedStateTransitionFees.feeRefundsSum,
            final: predictedStateTransitionFees.total,
            operations: predictedStateTransitionOperations.map((operation) => operation.toJSON()),
          },
          actual: {
            storage: actualStateTransitionFees.storageFee,
            processing: actualStateTransitionFees.processingFee,
            refunds: actualStateTransitionFees.feeRefundsSum,
            final: actualStateTransitionFees.total,
            operations: actualStateTransitionOperations.map((operation) => operation.toJSON()),
          },
        },
        txType: stateTransition.getType(),
      },
      `${stateTransition.constructor.name} execution took ${deliverTxTiming} seconds and cost ${actualStateTransitionFees.total} credits`,
    );

    let feeRefunds = {};
    if (actualStateTransitionFees.feeRefunds.length > 0) {
      feeRefunds = actualStateTransitionFees.feeRefunds[0].creditsPerEpoch;
    }

    return {
      code: 0,
      fees: {
        storageFee: actualStateTransitionFees.storageFee,
        processingFee: actualStateTransitionFees.processingFee,
        feeRefunds,
        feeRefundsSum: actualStateTransitionFees.feeRefundsSum,
      },
    };
  }

  return deliverTx;
}

module.exports = deliverTxFactory;
