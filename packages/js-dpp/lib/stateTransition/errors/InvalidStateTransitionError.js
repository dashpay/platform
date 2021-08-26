const DPPError = require('../../errors/DPPError');

class InvalidStateTransitionError extends DPPError {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawStateTransition} [rawStateTransition]
   */
  constructor(errors, rawStateTransition = undefined) {
    let message = `Invalid State Transition: "${errors[0].message}"`;
    if (errors.length > 1) {
      message = `${message} and ${errors.length - 1} more`;
    }

    super(message);

    this.errors = errors;
    this.rawStateTransition = rawStateTransition;
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
   * @return {RawStateTransition}
   */
  getRawStateTransition() {
    return this.rawStateTransition;
  }
}

module.exports = InvalidStateTransitionError;
