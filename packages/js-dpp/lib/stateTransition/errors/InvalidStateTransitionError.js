class InvalidStateTransitionError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawDataContractCreateTransition|RawDocumentsBatchTransition} rawStateTransition
   */
  constructor(errors, rawStateTransition) {
    super();

    this.name = this.constructor.name;
    this.message = `Invalid State Transition: "${errors[0].message}"`;
    if (errors.length > 1) {
      this.message = `${this.message} and ${errors.length - 1} more`;
    }

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
   * @return {RawDataContractCreateTransition|RawDocumentsBatchTransition}
   */
  getRawStateTransition() {
    return this.rawStateTransition;
  }
}

module.exports = InvalidStateTransitionError;
