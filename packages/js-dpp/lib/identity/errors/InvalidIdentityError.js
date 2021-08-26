const DPPError = require('../../errors/DPPError');

class InvalidIdentityError extends DPPError {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawIdentity} rawIdentity
   */
  constructor(errors, rawIdentity) {
    let message = `Invalid Identity: "${errors[0].message}"`;
    if (errors.length > 1) {
      message = `${message} and ${errors.length - 1} more`;
    }

    super(message);

    this.errors = errors;
    this.rawIdentity = rawIdentity;
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
   * Get raw Identity
   *
   * @return {RawIdentity}
   */
  getRawIdentity() {
    return this.rawIdentity;
  }
}

module.exports = InvalidIdentityError;
