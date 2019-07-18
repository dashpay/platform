class ValidationResult {
  /**
   * @param {ValidationError[]} [errors]
   */
  constructor(errors = []) {
    this.errors = errors;
  }

  /**
   * Add consensus error
   *
   * @param {...ValidationError} error
   */
  addError(...error) {
    this.errors.push(...error);

    return this;
  }

  /**
   * Get consensus errors
   *
   * @return {ValidationError[]}
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

    return this;
  }
}

module.exports = ValidationResult;
