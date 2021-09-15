const InvalidStateTransitionError = require('@dashevo/dpp/lib/stateTransition/errors/InvalidStateTransitionError');
const InvalidArgumentGrpcError = require('@dashevo/grpc-common/lib/server/error/InvalidArgumentGrpcError');

const DPPValidationError = require('../errors/DPPValidationError');

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

      throw new InvalidArgumentGrpcError('State Transition is not specified');
    }

    const stateTransitionSerialized = Buffer.from(stateTransitionByteArray);

    let stateTransition;
    try {
      stateTransition = await dpp
        .stateTransition
        .createFromBuffer(stateTransitionSerialized);
    } catch (e) {
      if (e instanceof InvalidStateTransitionError) {
        logger.info('Invalid state transition');
        logger.debug({
          consensusErrors: e.getErrors(),
        });

        throw new DPPValidationError('Invalid state transition', e.getErrors());
      }

      throw e;
    }

    let result = await dpp.stateTransition.validateSignature(stateTransition);

    if (!result.isValid()) {
      const consensusErrors = result.getErrors();

      logger.info('Invalid state transition signature');

      logger.debug({
        consensusErrors,
      });

      throw new DPPValidationError('Invalid state transition signature', consensusErrors);
    }

    result = await dpp.stateTransition.validateFee(stateTransition);

    if (!result.isValid()) {
      const consensusErrors = result.getErrors();

      logger.info('Insufficient funds to process state transition');

      logger.debug({
        consensusErrors,
      });

      throw new DPPValidationError('Insufficient funds to process state transition', consensusErrors);
    }

    return stateTransition;
  }

  return unserializeStateTransition;
}

module.exports = unserializeStateTransitionFactory;
