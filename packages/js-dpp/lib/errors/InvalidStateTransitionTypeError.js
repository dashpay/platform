const ConsensusError = require('./ConsensusError');

class InvalidStateTransitionTypeError extends ConsensusError {
  /**
   * @param {RawDataContractStateTransition|RawDocumentsStateTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super(`Invalid State Transition type ${rawStateTransition.type}`);

    this.rawStateTransition = rawStateTransition;
  }

  /**
   * Get raw State Transition
   *
   * @return {RawDataContractStateTransition|RawDocumentsStateTransition}
   */
  getRawStateTransition() {
    return this.rawStateTransition;
  }
}

module.exports = InvalidStateTransitionTypeError;
