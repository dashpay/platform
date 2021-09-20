const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');
const InvalidArgumentAbciError = require('../../errors/InvalidArgumentAbciError');

const DPPValidationAbciError = require('../../errors/DPPValidationAbciError');

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
      logger.info('State transition is not specified');

      throw new InvalidArgumentAbciError('State Transition is not specified');
    }

    const stateTransitionSerialized = Buffer.from(stateTransitionByteArray);

    let stateTransition;
    try {
      stateTransition = await dpp
        .stateTransition
        .createFromBuffer(stateTransitionSerialized);
    } catch (e) {
      if (e instanceof InvalidStateTransitionError) {
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

    result = await dpp.stateTransition.validateFee(stateTransition);

    if (!result.isValid()) {
      const consensusError = result.getFirstError();
      const message = 'Insufficient funds to process state transition';

      logger.info(message);

      logger.debug({
        consensusError,
      });

      throw new DPPValidationAbciError(message, consensusError);
    }

    return stateTransition;
  }

  return unserializeStateTransition;
}

module.exports = unserializeStateTransitionFactory;
