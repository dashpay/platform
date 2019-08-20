class DataTriggerExecutionResult {
  /**
   * @param {DataTriggerExecutionError[]} errors
   */
  constructor(errors = []) {
    this.errors = errors;
  }

  /**
   * Add an error to result
   *
   * @param {DataTriggerExecutionError} error
   */
  addError(...error) {
    this.errors.push(...error);
  }

  /**
   * Get all data trigger execution errors
   *
   * @return {DataTriggerExecutionError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Check if result have no errros
   *
   * @return {boolean}
   */
  isOk() {
    return this.errors.length === 0;
  }
}

module.exports = DataTriggerExecutionResult;
