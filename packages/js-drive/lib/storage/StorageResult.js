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
   * @param {T} value
   */
  setValue(value) {
    this.#result = value;
  }

  /**
   * @return {AbstractOperation[]}
   */
  getOperations() {
    return this.#operations;
  }

  /**
   * @return {boolean}
   */
  isNull() {
    return this.#result === null || this.#result === undefined;
  }

  /**
   * @return {boolean}
   */
  isEmpty() {
    return this.isNull() || (Array.isArray(this.#result) && this.#result.length === 0);
  }

  /**
   * @param {AbstractOperation} operation
   */
  addOperation(...operation) {
    this.#operations.push(...operation);
  }
}

module.exports = StorageResult;
