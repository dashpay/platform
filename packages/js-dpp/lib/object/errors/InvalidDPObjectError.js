class InvalidDPObjectError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {Object} rawDPObject
   */
  constructor(errors, rawDPObject) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid DPObject';

    this.errors = errors;
    this.rawDPObject = rawDPObject;

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
   * Get raw DPObject
   *
   * @return {Object}
   */
  getRawDPObject() {
    return this.rawDPObject;
  }
}

module.exports = InvalidDPObjectError;
