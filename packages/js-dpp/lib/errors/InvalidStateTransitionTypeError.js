const ConsensusError = require('./ConsensusError');

class InvalidStateTransitionTypeError extends ConsensusError {
  /**
   * @param {RawStateTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super(`Invalid State Transition type ${rawStateTransition.type}`);

    this.rawStateTransition = rawStateTransition;
  }

  /**
   * Get raw State Transition
   *
   * @return {RawStateTransition}
   */
  getRawStateTransition() {
    return this.rawStateTransition;
  }
}

module.exports = InvalidStateTransitionTypeError;
