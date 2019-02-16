const Emittery = require('emittery');

class BlockchainReaderMediator extends Emittery {
  /**
   * @param {BlockchainReaderState} state
   * @param {number} initialBlockHeight
   */
  constructor(state, initialBlockHeight) {
    super();

    this.state = state;
    this.initialBlockHeight = initialBlockHeight;
  }

  /**
   * @return {BlockchainReaderState}
   */
  getState() {
    return this.state;
  }

  /**
   * @return {number}
   */
  getInitialBlockHeight() {
    return this.initialBlockHeight;
  }

  /**
   * Reset reader
   *
   * @return {Promise<void>}
   */
  async reset() {
    this.getState().clear();

    await this.emitSerial(BlockchainReaderMediator.EVENTS.RESET);
  }
}

BlockchainReaderMediator.EVENTS = {
  FULLY_SYNCED: 'fullySynced',
  OUT_OF_BOUNDS: 'outOfBounds',
  BEGIN: 'begin',
  STATE_TRANSITION: 'stateTransition',
  STATE_TRANSITION_ORPHANED: 'stateTransitionOrphaned',
  STATE_TRANSITION_SKIP: 'stateTransitionSkip',
  STATE_TRANSITION_ERROR: 'stateTransitionError',
  DP_CONTRACT_APPLIED: 'dpContractApplied',
  DP_CONTRACT_REVERTED: 'dpContractReverted',
  DP_CONTRACT_MARKED_DELETED: 'dpContractMarkedDeleted',
  DP_OBJECT_APPLIED: 'dpObjectApplied',
  DP_OBJECT_REVERTED: 'dpObjectReverted',
  DP_OBJECT_MARKED_DELETED: 'dpObjectMarkedDeleted',
  BLOCK_BEGIN: 'blockBegin',
  BLOCK_ORPHANED: 'blockOrphaned',
  BLOCK_END: 'blockEnd',
  BLOCK_ERROR: 'blockError',
  BLOCK_SEQUENCE_VALIDATION_IMPOSSIBLE: 'blockSequenceValidationImpossible',
  RESET: 'reset',
  END: 'end',
};

module.exports = BlockchainReaderMediator;
