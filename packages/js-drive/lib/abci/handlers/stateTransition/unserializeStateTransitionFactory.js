const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

const DPPValidationAbciError = require('../../errors/DPPValidationAbciError');

const TIMERS = require('../timers');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {Object} noopLogger
 * @return {unserializeStateTransition}
 */
function unserializeStateTransitionFactory(dpp, noopLogger, dppWasm) {
  /**
   * @typedef unserializeStateTransition
   * @param {Uint8Array} stateTransitionByteArray
   * @param {Object} [options]
   * @param {BaseLogger} [options.logger]
   * @param {ExecutionTimer} [options.executionTimer]
   * @return {AbstractStateTransition}
   */
  async function unserializeStateTransition(stateTransitionByteArray, options = {}) {
    // either use a logger passed or use noop logger
    const logger = (options.logger || noopLogger);

    // measure timing if timer is passed
    const executionTimer = (options.executionTimer || {
      startTimer: () => {},
      stopTimer: () => {},
    });

    if (!stateTransitionByteArray) {
      logger.warn('State transition is not specified');

      throw new InvalidArgumentAbciError('State Transition is not specified');
    }

    const stateTransitionSerialized = Buffer.from(stateTransitionByteArray);

    executionTimer.startTimer(TIMERS.DELIVER_TX.VALIDATE_BASIC);

    let stateTransition;
    try {
      stateTransition = await dpp
        .stateTransition
        .createFromBuffer(stateTransitionSerialized);
    } catch (e) {
      if (e instanceof dppWasm.InvalidStateTransitionError) {
        const consensusError = e.getErrors()[0];
        const message = 'Invalid state transition';

        logger.info(message);
        logger.debug({
          consensusError,
        });

        throw new DPPValidationAbciError(message, consensusError);
      }

      throw e;
    }

    executionTimer.stopTimer(TIMERS.DELIVER_TX.VALIDATE_BASIC, true);

    executionTimer.startTimer(TIMERS.DELIVER_TX.VALIDATE_SIGNATURE);

    let result = await dpp.stateTransition.validateSignature(stateTransition);

    if (!result.isValid()) {
      const consensusError = result.getFirstError();
      const message = 'Invalid state transition signature';

      logger.info(message);

      logger.debug({
        consensusError,
      });

      throw new DPPValidationAbciError(message, consensusError);
    }

    executionTimer.stopTimer(TIMERS.DELIVER_TX.VALIDATE_SIGNATURE, true);

    executionTimer.startTimer(TIMERS.DELIVER_TX.VALIDATE_FEE);

    const executionContext = stateTransition.getExecutionContext();

    // Pre-calculate fee for validateState and state transition apply
    // with worst case costs to validate the whole state transition execution cost
    executionContext.enableDryRun();
    console.log('[Unserialize ST Factory] Validate state');

    await dpp.stateTransition.validateState(stateTransition);
    console.log('[Unserialize ST Factory] Validated st state');
    await dpp.stateTransition.apply(stateTransition);
    console.log('[Unserialize ST Factory] Applied st');

    executionContext.disableDryRun();

    result = await dpp.stateTransition.validateFee(stateTransition);
    console.log('[Unserialize ST Factory] Validated fee', result.isValid());

    if (!result.isValid()) {
      const consensusError = result.getFirstError();
      const message = 'Insufficient funds to process state transition';

      logger.info(message);

      logger.debug({
        consensusError,
      });

      throw new DPPValidationAbciError(message, consensusError);
    }

    executionTimer.stopTimer(TIMERS.DELIVER_TX.VALIDATE_FEE, true);
    console.log('[Unserialize ST Factory] Unserialized ST factory!');
    return stateTransition;
  }

  return unserializeStateTransition;
}

module.exports = unserializeStateTransitionFactory;
