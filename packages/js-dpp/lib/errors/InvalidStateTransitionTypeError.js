const ConsensusError = require('./ConsensusError');

class InvalidStateTransitionTypeError extends ConsensusError {
  /**
   * @param {RawDataContractCreateTransition|RawDocumentsBatchTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super(`Invalid State Transition type ${rawStateTransition.type}`);

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

module.exports = InvalidStateTransitionTypeError;
