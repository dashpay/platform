const ConsensusError = require('./ConsensusError');

class MissingStateTransitionTypeError extends ConsensusError {
  constructor(rawStateTransition) {
    super('type is not present');

    this.rawStateTransition = rawStateTransition;
  }

  /**
   * Get raw State Transition
   *
   * @return {RawDataContractCreateTransition}
   */
  getRawStateTransition() {
    return this.rawStateTransition;
  }
}

module.exports = MissingStateTransitionTypeError;
