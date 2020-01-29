const ConsensusError = require('./ConsensusError');

class UnknownIdentityTypeError extends ConsensusError {
  /**
   * @param {number} type
   */
  constructor(type) {
    super(`Identity type ${type} is within the reserved types range, but is unknown to the`
      + ' protocol');

    this.type = type;
  }

  /**
   * Get unknown type
   *
   * @return {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = UnknownIdentityTypeError;
