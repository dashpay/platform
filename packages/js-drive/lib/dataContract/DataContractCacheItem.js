class DataContractCacheItem {
  /**
   * @type {DataContract}
   */
  #dataContract;

  /**
   * @type {AbstractOperation[]}
   */
  #operations;

  /**
   *
   * @param {DataContract} dataContract
   * @param {AbstractOperation[]} operations
   */
  constructor(dataContract, operations) {
    this.#dataContract = dataContract;
    this.#operations = operations;
  }

  /**
   * @return {DataContract}
   */
  getDataContract() {
    return this.#dataContract;
  }

  /**
   * @return {AbstractOperation[]}
   */
  getOperations() {
    return this.#operations;
  }

  /**
   * @return {string}
   */
  getKey() {
    return this.#dataContract.getId().toString();
  }
}

module.exports = DataContractCacheItem;
