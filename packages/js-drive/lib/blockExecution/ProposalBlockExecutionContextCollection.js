class ProposalBlockExecutionContextCollection {
  constructor() {
    this.collection = new Map();
  }

  /**
   * Add BlockExecutionContext for the round
   *
   * @param {number} round
   * @param {BlockExecutionContext} context
   * @return {ProposalBlockExecutionContextCollection}
   */
  add(round, context) {
    this.collection.set(round, context);

    return this;
  }

  /**
   * Get execution context for the round
   *
   * @param {number} round
   * @return {BlockExecutionContext}
   */
  get(round) {
    return this.collection.get(round);
  }

  /**
   * Clear execution contexts
   * @return {ProposalBlockExecutionContextCollection}
   */
  clear() {
    this.collection.clear();

    return this;
  }
}

module.exports = ProposalBlockExecutionContextCollection;
