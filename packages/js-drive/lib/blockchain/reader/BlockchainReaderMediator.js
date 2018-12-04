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
  STATE_TRANSITION_STALE: 'stateTransitionStale',
  STATE_TRANSITION_SKIP: 'stateTransitionSkip',
  STATE_TRANSITION_ERROR: 'stateTransitionError',
  DAP_CONTRACT_APPLIED: 'dapContractApplied',
  DAP_CONTRACT_REVERTED: 'dapContractReverted',
  DAP_CONTRACT_MARKED_DELETED: 'dapContractMarkedDeleted',
  DAP_OBJECT_APPLIED: 'dapObjectApplied',
  DAP_OBJECT_REVERTED: 'dapObjectReverted',
  DAP_OBJECT_MARKED_DELETED: 'dapObjectMarkedDeleted',
  BLOCK_BEGIN: 'blockBegin',
  BLOCK_STALE: 'blockStale',
  BLOCK_END: 'blockEnd',
  BLOCK_ERROR: 'blockError',
  BLOCK_SEQUENCE_VALIDATION_IMPOSSIBLE: 'blockSequenceValidationImpossible',
  RESET: 'reset',
  END: 'end',
};

module.exports = BlockchainReaderMediator;
