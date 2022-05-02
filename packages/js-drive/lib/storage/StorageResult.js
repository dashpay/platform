/**
 * @template T
 */
class StorageResult {
  /**
   * @type {T}
   */
  #value;

  /**
   * @type {AbstractOperation[]}
   */
  #operations;

  /**
   * @template T
   * @param {T} value
   * @param {AbstractOperation[]} operations
   */
  constructor(value, operations = []) {
    this.#value = value;
    this.#operations = operations;
  }

  /**
   * @return {T}
   */
  getValue() {
    return this.#value;
  }

  /**
   * @param {T} value
   */
  setValue(value) {
    this.#value = value;
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
    return this.#value === null || this.#value === undefined;
  }

  /**
   * @return {boolean}
   */
  isEmpty() {
    return this.isNull() || (Array.isArray(this.#value) && this.#value.length === 0);
  }

  /**
   * @param {AbstractOperation} operation
   */
  addOperation(...operation) {
    this.#operations.push(...operation);
  }
}

module.exports = StorageResult;
