class DataTriggerExecutionResult {
  /**
   * @param {AbstractDataTriggerError[]} errors
   */
  constructor(errors = []) {
    this.errors = errors;
  }

  /**
   * Add an error to result
   *
   * @param {AbstractDataTriggerError} error
   */
  addError(...error) {
    this.errors.push(...error);
  }

  /**
   * Get all data trigger execution errors
   *
   * @return {AbstractDataTriggerError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Check if result have no errors
   *
   * @return {boolean}
   */
  isOk() {
    return this.errors.length === 0;
  }
}

module.exports = DataTriggerExecutionResult;
