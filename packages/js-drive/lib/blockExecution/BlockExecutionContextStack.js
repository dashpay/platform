const ContextsAreMoreThanStackMaxSizeError = require("./errors/ContextsAreMoreThanStackMaxSizeError");

class BlockExecutionContextStack {
  /**
   * @type {BlockExecutionContext[]}
   */
  #contexts = [];

  /**
   * @type {number}
   */
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
   * Get last context according to stack maximum size
   *
   * @returns {BlockExecutionContext}
   */
  getLast() {
    return this.#contexts[this.#maxSize - 1];
  }

  /**
   * Get the latest context from the stack
   *
   * @return {BlockExecutionContext}
   */
  getLatest() {
    return this.#contexts[this.#contexts.length - 1];
  }

  /**
   * Remove the last context from the stack
   *
   * @return {BlockExecutionContextStack}
   */
  removeLatest() {
    this.#contexts.pop();

    return this;
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
