class ValidationResult {
  /**
   * @param {AbstractConsensusError[]} [errors]
   */
  constructor(errors = []) {
    this.errors = errors;
    this.data = undefined;
  }

  /**
   * Add consensus error
   *
   * @param {...AbstractConsensusError} error
   */
  addError(...error) {
    this.errors.push(...error);
  }

  /**
   * Get consensus errors
   *
   * @return {AbstractConsensusError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Get the first consensus error
   *
   * @returns {AbstractConsensusError}
   */
  getFirstError() {
    return this.errors[0];
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

  /**
   *
   * @param {*} data
   * @return {ValidationResult}
   */
  setData(data) {
    this.data = data;

    return this;
  }

  /**
   *
   * @return {*}
   */
  getData() {
    return this.data;
  }
}

module.exports = ValidationResult;
