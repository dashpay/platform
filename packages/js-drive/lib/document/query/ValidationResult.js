class ValidationResult {
  /**
   * @param {ValidationError[]} [errors]
   */
  constructor(errors = []) {
    this.errors = errors;
  }

  /**
   * Add error
   *
   * @param {...ValidationError} error
   */
  addError(...error) {
    this.errors.push(...error);

    return this;
  }

  /**
   * Get errors
   *
   * @return {ValidationError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Is valid
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
