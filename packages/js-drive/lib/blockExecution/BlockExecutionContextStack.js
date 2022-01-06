const ContextsAreMoreThanStackMaxSizeError = require("./errors/ContextsAreMoreThanStackMaxSizeError");

class BlockExecutionContextStack {
  #contexts = [];
  #maxSize = 3;

  /**
   *
   * @param {BlockExecutionContext[]} contexts
   */
  setContexts(contexts) {
    if (contexts.length > this.#maxSize) {
      throw new ContextsAreMoreThanStackMaxSizeError();
    }

    this.#contexts = contexts;
  }

  /**
   *
   * @return {BlockExecutionContext[]}
   */
  getContexts() {
    return this.#contexts;
  }

  /**
   * @returns {BlockExecutionContext}
   */
  getFirst() {
    return this.#contexts[0];
  }

  /**
   * @returns {BlockExecutionContext}
   */
  getLast() {
    return this.#contexts[this.#maxSize - 1];
  }

  /**
   * @param {BlockExecutionContext} context
   */
  add(context) {
    this.#contexts.unshift(context);

    if (this.#contexts.length > this.#maxSize) {
      this.#contexts.pop();
    }
  }

  /**
   * @return {number}
   */
  getSize() {
    return this.#contexts.length;
  }
}

module.exports = BlockExecutionContextStack;
