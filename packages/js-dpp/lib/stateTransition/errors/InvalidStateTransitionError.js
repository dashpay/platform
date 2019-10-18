class InvalidStateTransitionError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawDataContractStateTransition|RawDocumentsStateTransition} rawStateTransition
   */
  constructor(errors, rawStateTransition) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid State Transition';

    this.errors = errors;
    this.rawStateTransition = rawStateTransition;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get validation errors
   *
   * @return {ConsensusError[]}
   */
  getErrors() {
    return this.errors;
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

module.exports = InvalidStateTransitionError;
