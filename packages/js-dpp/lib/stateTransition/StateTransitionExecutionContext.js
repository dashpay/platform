class StateTransitionExecutionContext {
  constructor() {
    /**
     * @type {AbstractOperation[]}
     */
    this.operations = [];
  }

  /**
   * Add operation into context
   *
   * @param {AbstractOperation} operation
   */
  addOperation(...operation) {
    this.operations.push(...operation);
  }

  /**
   * Set operations into context
   *
   * @param {AbstractOperation[]} operations
   */
  setOperations(operations) {
    this.operations = operations;
  }

  /**
   * Get operations
   *
   * @return {AbstractOperation[]}
   */
  getOperations() {
    return this.operations;
  }
}

module.exports = StateTransitionExecutionContext;
