class ValidationResult {
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
   * Is data valid
   *
   * @return {boolean}
   */
  isValid() {
    return !this.errors.length;
  }

  /**
   * Merge Validation results
   *
   * @param {ValidationResult} result
   */
  merge(result) {
    if (!result.isValid()) {
      this.addError(...result.getErrors());
    }
  }
}

module.exports = ValidationResult;
