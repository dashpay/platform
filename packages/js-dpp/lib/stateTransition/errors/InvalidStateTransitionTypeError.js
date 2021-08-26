const DPPError = require('../../errors/DPPError');

class InvalidStateTransitionTypeError extends DPPError {
  /**
   * @param {number} type
   */
  constructor(type) {
    super(`Invalid State Transition type ${type}`);

    this.type = type;
  }

  /**
   * Get State Transition type
   *
   * @return {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = InvalidStateTransitionTypeError;
