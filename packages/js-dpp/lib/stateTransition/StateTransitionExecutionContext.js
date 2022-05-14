class StateTransitionExecutionContext {
  constructor() {
    /**
     * @type {AbstractOperation[]}
     */
    this.operations = [];
    this.dryRun = false;
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
   * Enable dry run
   *
   * Count only operations
   */
  enableDryRun() {
    this.dryRun = true;
  }

  /**
   * Disable dry run
   *
   * Execute state transition
   */
  disableDryRun() {
    this.dryRun = false;
  }

  isDryRun() {
    return this.dryRun;
  }
}

module.exports = StateTransitionExecutionContext;
