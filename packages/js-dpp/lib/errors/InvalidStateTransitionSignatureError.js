const ConsensusError = require('./ConsensusError');

class InvalidStateTransitionSignatureError extends ConsensusError {
  /**
   * @param {DataContractStateTransition|DocumentsStateTransition} stateTransition
   */
  constructor(stateTransition) {
    super(`Invalid State Transition signature ${stateTransition.getSignature()}`);

    this.stateTransition = stateTransition;
  }

  /**
   * Get State Transition
   *
   * @return {DataContractStateTransition|DocumentsStateTransition}
   */
  getRawStateTransition() {
    return this.stateTransition;
  }
}

module.exports = InvalidStateTransitionSignatureError;
