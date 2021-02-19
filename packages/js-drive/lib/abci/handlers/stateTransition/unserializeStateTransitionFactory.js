const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');
const BalanceIsNotEnoughError = require('@dashevo/dpp/lib/errors/BalanceIsNotEnoughError');

const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');
const InsufficientFundsError = require('../../errors/InsufficientFundsError');

/**
 * @param {DashPlatformProtocol} dpp
 * @param {Object} noopLogger
 * @return {unserializeStateTransition}
 */
function unserializeStateTransitionFactory(dpp, noopLogger) {
  /**
   * @typedef unserializeStateTransition
   * @param {Uint8Array} stateTransitionByteArray
   * @param {Object} [options]
   * @param {BaseLogger} [options.logger]
   * @return {DocumentsBatchTransition|DataContractCreateTransition|IdentityCreateTransition}
   */
  async function unserializeStateTransition(stateTransitionByteArray, options = {}) {
    // either use a logger passed or use noop logger
    const logger = (options.logger || noopLogger);

    if (!stateTransitionByteArray) {
      const error = new InvalidArgumentAbciError('State Transition is not specified');

      logger.info('State transition is not specified');

      throw error;
    }

    const stateTransitionSerialized = Buffer.from(stateTransitionByteArray);

    let stateTransition;
    try {
      stateTransition = await dpp
        .stateTransition
        .createFromBuffer(stateTransitionSerialized);
    } catch (e) {
      if (e instanceof InvalidStateTransitionError) {
        const error = new InvalidArgumentAbciError('State Transition is invalid', { errors: e.getErrors() });

        logger.info('State transition structure is invalid');
        logger.debug({
          consensusErrors: e.getErrors(),
        });

        throw error;
      }

      throw e;
    }

    const result = await dpp.stateTransition.validateFee(stateTransition);

    if (!result.isValid()) {
      const consensusErrors = result.getErrors();

      let error;

      if (consensusErrors.length === 1 && consensusErrors[0] instanceof BalanceIsNotEnoughError) {
        error = new InsufficientFundsError(consensusErrors[0].getBalance());

        logger.info('Insufficient funds to process state transition');
      } else {
        error = new InvalidArgumentAbciError('State Transition is invalid', { errors: consensusErrors });

        logger.info('State transition structure is invalid');
      }

      logger.debug({
        consensusErrors,
      });

      throw error;
    }

    return stateTransition;
  }

  return unserializeStateTransition;
}

module.exports = unserializeStateTransitionFactory;
