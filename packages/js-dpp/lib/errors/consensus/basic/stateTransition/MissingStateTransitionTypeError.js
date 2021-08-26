const AbstractBasicError = require('../AbstractBasicError');

class MissingStateTransitionTypeError extends AbstractBasicError {
  /**
   * @param {RawStateTransition} rawStateTransition
   */
  constructor(rawStateTransition) {
    super('type is not present');

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

module.exports = MissingStateTransitionTypeError;
