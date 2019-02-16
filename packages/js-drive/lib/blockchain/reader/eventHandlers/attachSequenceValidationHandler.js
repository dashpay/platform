const ReaderMediator = require('../BlockchainReaderMediator');

const RestartBlockchainReaderError = require('../errors/RestartBlockchainReaderError');

const WrongSequenceError = require('./errors/WrongSequenceError');
const NotAbleToValidateSequenceError = require('./errors/NotAbleToValidateSequenceError');

/**
 *
 * @param {BlockchainReaderMediator} readerMediator
 * @param {createStateTransitionsFromBlock} createStateTransitions
 */
module.exports = function attachSequenceValidationHandler(readerMediator, createStateTransitions) {
  /**
   * @param {object} currentBlock
   * @param {object} previousBlock
   * @return {boolean}
   */
  function isNotAbleToValidateSequence(currentBlock, previousBlock) {
    const firstSyncedBlockHeight = readerMediator.getState().getFirstBlockHeight();

    if (!previousBlock) {
      if (currentBlock.height !== readerMediator.getInitialBlockHeight()) {
        // The state doesn't contain synced blocks and
        // current block's height is not initial blocks height
        return true;
      }
    } else if (currentBlock.height <= firstSyncedBlockHeight) {
      // The state does not have previous block to rely onto
      return true;
    }

    return false;
  }

  /**
   * @param {{previousblockhash, hash}} currentBlock
   * @param {{previousblockhash, hash}} previousBlock
   * @return {boolean}
   */
  function isWrongSequence(currentBlock, previousBlock) {
    return previousBlock
      && currentBlock.previousblockhash
      && currentBlock.previousblockhash !== previousBlock.hash;
  }

  /**
   * @param {object} block
   */
  function validateBlockSequence(block) {
    const lastSyncedBlock = readerMediator.getState().getLastBlock();

    // Do we have enough synced blocks to verify sequence?
    if (isNotAbleToValidateSequence(block, lastSyncedBlock)) {
      throw new NotAbleToValidateSequenceError();
    }

    // Is sequence correct?
    if (isWrongSequence(block, lastSyncedBlock)) {
      throw new WrongSequenceError();
    }
  }

  /**
   * @param {Error} error
   * @param {object} block
   * @return {Promise<void>}
   */
  async function restartReaderIfSequenceIsWrong({ error, block }) {
    const lastSyncedBlock = readerMediator.getState().getLastBlock();
    const firstSyncedBlockHeight = readerMediator.getState().getFirstBlockHeight();

    if (error instanceof NotAbleToValidateSequenceError) {
      await readerMediator.emitSerial(
        ReaderMediator.EVENTS.BLOCK_SEQUENCE_VALIDATION_IMPOSSIBLE,
        {
          height: block.height,
          firstSyncedBlockHeight,
        },
      );

      await readerMediator.reset();

      throw new RestartBlockchainReaderError(readerMediator.getInitialBlockHeight());
    }

    // Restart iterator if block sequence is wrong
    if (error instanceof WrongSequenceError) {
      // Restart iterator from the next block after the last synced
      // if current block height greater than 1
      if (block.height > lastSyncedBlock.height + 1) {
        throw new RestartBlockchainReaderError(lastSyncedBlock.height + 1);
      }

      readerMediator.getState().removeLastBlock();

      // Mark block as orphaned
      await readerMediator.emitSerial(
        ReaderMediator.EVENTS.BLOCK_ORPHANED,
        lastSyncedBlock,
      );

      // Mark State Transitions from block as orphaned
      const orphanedStateTransitions = await createStateTransitions(lastSyncedBlock);

      for (const orphanedStateTransition of orphanedStateTransitions.reverse()) {
        await readerMediator.emitSerial(ReaderMediator.EVENTS.STATE_TRANSITION_ORPHANED, {
          stateTransition: orphanedStateTransition,
          block: lastSyncedBlock,
        });
      }

      // Calculate next block height
      let nextBlockHeight = block.height - 1;

      // if the previous synced block is higher then stay with the current block from chain
      if (lastSyncedBlock.height >= block.height) {
        nextBlockHeight = block.height;
      }

      throw new RestartBlockchainReaderError(nextBlockHeight);
    }
  }

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_BEGIN, validateBlockSequence);
  readerMediator.on(ReaderMediator.EVENTS.BLOCK_ERROR, restartReaderIfSequenceIsWrong);
};
