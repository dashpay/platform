const ConsensusError = require('./ConsensusError');

class MismatchSTDocumentsAndActionsError extends ConsensusError {
  constructor(rawStateTransition) {
    super('Mismatch documents and actions count');

    this.rawStateTransition = rawStateTransition;
  }

  /**
   * Get raw State Transition
   *
   * @return {RawDocumentsStateTransition}
   */
  getRawStateTransition() {
    return this.rawStateTransition;
  }
}

module.exports = MismatchSTDocumentsAndActionsError;
