class StateTransitionExecutionContext {
  constructor() {
    /**
     * @type {AbstractOperation[]}
     */
    this.actualOperations = [];
    /**
     * @type {AbstractOperation[]}
     */
    this.dryOperations = [];
    this.dryRun = false;
    this.lastCalculatedFees = null;
  }

  /**
   * Add operation into context
   *
   * @param {AbstractOperation} operation
   */
  addOperation(...operation) {
    if (this.isDryRun()) {
      this.dryOperations.push(...operation);
    } else {
      this.actualOperations.push(...operation);
    }
  }

  /**
   * Get operations
   *
   * @return {AbstractOperation[]}
   */
  getOperations() {
    return this.actualOperations.concat(this.dryOperations);
  }

  /**
   * Clear dry operations
   */
  clearDryOperations() {
    this.dryOperations = [];
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

  /**
   * @return {boolean}
   */
  isDryRun() {
    return this.dryRun;
  }

  /**
   * @return {{
   *   storageFee: number,
   *   processingFee: number,
   *   feeRefunds: {identifier: Buffer, creditsPerEpoch: Object<string, number>}[],
   *   feeRefundsSum: number,
   *   total: number,
   * }|null}
   */
  getLastCalculatedFeeDetails() {
    return this.lastCalculatedFees;
  }

  /**
   * @param {{
   *   storageFee:number,
   *   processingFee:number,
   *   feeRefunds: {identifier: Buffer, creditsPerEpoch: Object<string, number>}[],
   *   feeRefundsSum: number,
   *   total: number,
   * }} lastCalculatedFees
   */
  setLastCalculatedFeeDetails(lastCalculatedFees) {
    this.lastCalculatedFees = lastCalculatedFees;
  }
}

module.exports = StateTransitionExecutionContext;
