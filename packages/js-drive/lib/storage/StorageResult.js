/**
 * @template T
 */
class StorageResult {
  /**
   * @type {T}
   */
  #result;

  /**
   * @type {AbstractOperation[]}
   */
  #operations;

  /**
   * @template T
   * @param {T} result
   * @param {AbstractOperation[]} operations
   */
  constructor(result, operations = []) {
    this.#result = result;
    this.#operations = operations;
  }

  /**
   * @return {T}
   */
  getResult() {
    return this.#result;
  }

  /**
   * @return {AbstractOperation[]}
   */
  getOperations() {
    return this.#operations;
  }

  /**
   * @param {AbstractOperation} operation
   */
  addOperation(...operation) {
    this.#operations.push(...operation);
  }
}

module.exports = StorageResult;
