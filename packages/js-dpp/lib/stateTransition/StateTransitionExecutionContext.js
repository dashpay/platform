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

  /**
   * Merge too contexts
   *
   * @param context
   */
  merge(context) {
    this.operations.concat(context.operations);
  }

  /**
   * Clone context
   *
   * @return {StateTransitionExecutionContext}
   */
  clone() {
    const context = new StateTransitionExecutionContext();

    context.setOperations([...this.getOperations()]);

    return context;
  }
}

module.exports = StateTransitionExecutionContext;
