class StateTransitionExecutionContext {
  constructor() {
    /**
     * @type {AbstractOperation[]}
     */
    this.operations = [];
  }

  /**
   * Save operation into context
   *
   * @param {AbstractOperation} operation
   */
  addOperation(...operation) {
    this.operations.push(...operation);
  }

  /**
   * Merge too contexts
   *
   * @param context
   */
  merge(context) {
    this.operations.concat(context.operations);
  }
}

module.exports = StateTransitionExecutionContext;
