class InvalidIdentityError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawIdentity} rawIdentity
   */
  constructor(errors, rawIdentity) {
    super();

    this.name = this.constructor.name;
    this.message = `Invalid Identity: "${errors[0].message}"`;
    if (errors.length > 1) {
      this.message = `${this.message} and ${errors.length - 1} more`;
    }

    this.errors = errors;
    this.rawIdentity = rawIdentity;

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
   * Get raw Identity
   *
   * @return {RawIdentity}
   */
  getRawIdentity() {
    return this.rawIdentity;
  }
}

module.exports = InvalidIdentityError;
