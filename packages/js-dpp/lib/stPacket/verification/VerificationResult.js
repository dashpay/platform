class VerificationResult {
  /**
   * @param {ConsensusError[]} [errors]
   */
  constructor(errors = []) {
    this.errors = errors;
  }

  /**
   * Add consensus error
   *
   * @param {...ConsensusError} error
   */
  addError(...error) {
    this.errors.push(...error);
  }

  /**
   * Get consensus errors
   *
   * @return {ConsensusError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Is ST Packet data valid
   *
   * @return {boolean}
   */
  isValid() {
    return !this.errors.length;
  }
}

module.exports = VerificationResult;
