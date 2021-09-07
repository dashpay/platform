const AbstractBasicError = require('../AbstractBasicError');

class InvalidStateTransitionTypeError extends AbstractBasicError {
  /**
   * @param {number} type
   */
  constructor(type) {
    super(`Invalid State Transition type ${type}`);

    this.type = type;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get state transition type
   *
   * @return {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = InvalidStateTransitionTypeError;
