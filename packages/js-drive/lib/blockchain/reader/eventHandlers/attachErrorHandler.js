const ReaderMediator = require('../BlockchainReaderMediator');
const IgnoreStateTransitionError = require('../errors/IgnoreStateTransitionError');

/**
 * @param {BlockchainReaderMediator} readerMediator
 * @param {{skipStateTransitionWithErrors: boolean}} options
 */
module.exports = function attachErrorHandler(readerMediator, options) {
  const { skipStateTransitionWithErrors } = Object.assign({
    skipStateTransitionWithErrors: false,
  }, options);

  async function handleStateTransitionError({ block, stateTransition }) {
    // If we want to skip block processing with errors
    if (skipStateTransitionWithErrors) {
      await readerMediator.emitSerial(
        ReaderMediator.EVENTS.STATE_TRANSITION_SKIP, {
          block,
          stateTransition,
        },
      );

      throw new IgnoreStateTransitionError();
    }
  }

  readerMediator.on(ReaderMediator.EVENTS.STATE_TRANSITION_ERROR, handleStateTransitionError);
};
