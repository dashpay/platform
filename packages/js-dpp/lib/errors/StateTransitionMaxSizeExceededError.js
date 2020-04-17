const ConsensusError = require('./ConsensusError');

class StateTransitionMaxSizeExceededError extends ConsensusError {
  /**
   * @param {RawDataContractCreateTransition|RawDocumentsBatchTransition} rawStateTransition
   * @param {number} maxSizeKBytes
   */
  constructor(rawStateTransition, maxSizeKBytes) {
    super(`State transition size is more than ${maxSizeKBytes}kb`);

    this.rawStateTransition = rawStateTransition;
  }

  /**
   * Get raw State Transition
   *
   * @return {RawDataContractCreateTransition|RawDocumentsBatchTransition}
   */
  getRawStateTransition() {
    return this.rawStateTransition;
  }
}

module.exports = StateTransitionMaxSizeExceededError;
