const ReaderMediator = require('../BlockchainReaderMediator');
const RestartBlockchainReaderError = require('../errors/RestartBlockchainReaderError');
const IgnoreStateTransitionError = require('../errors/IgnoreStateTransitionError');

/**
 * @param {BlockchainReaderMediator} readerMediator
 * @param {{skipBlockWithErrors: boolean}} options
 */
module.exports = function attachErrorHandler(readerMediator, options) {
  const { skipBlockWithErrors, skipStateTransitionWithErrors } = Object.assign({
    skipBlockWithErrors: false,
    skipStateTransitionWithErrors: false,
  }, options);

  /**
   * @param {{ error: Error, block: object, stateTransition: StateTransitionHeader}} eventPayload
   */
  async function handleBlockError({ block }) {
    // If we want to skip block processing with errors
    if (skipBlockWithErrors) {
      await readerMediator.emitSerial(ReaderMediator.EVENTS.BLOCK_SKIP, block);

      // Move iterator to the next block
      // if we still have something to read
      if (block.nextblockhash) {
        throw new RestartBlockchainReaderError(block.height + 1);
      }
    }
  }

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
  readerMediator.on(ReaderMediator.EVENTS.BLOCK_ERROR, handleBlockError);
};
