const BlockExecutionContextNotFoundError = require('../abci/errors/BlockExecutionContextNotFoundError');

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
    const result = this.collection.get(round);

    if (!result) {
      throw new BlockExecutionContextNotFoundError(round);
    }

    return result;
  }

  /**
   * Clear execution contexts
   * @return {ProposalBlockExecutionContextCollection}
   */
  clear() {
    this.collection.clear();

    return this;
  }

  /**
   * Check if collection contains contexts
   *
   * @return {boolean}
   */
  isEmpty() {
    return this.collection.size === 0;
  }
}

module.exports = ProposalBlockExecutionContextCollection;
