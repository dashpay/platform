class AbstractDataProvider {
  /**
   * Fetch Dap Contract
   *
   * @param {string} id
   * @return {DapContract|null}
   */
  // eslint-disable-next-line no-unused-vars
  fetchDapContract(id) {
    throw new Error('Not implemented');
  }

  /**
   * Fetch Dap Objects
   *
   * @param {[{type: string, primaryKey: string}]} primaryKeysAndTypes
   * @return {DapObject[]}
   */
  // eslint-disable-next-line no-unused-vars
  fetchDapObjects(primaryKeysAndTypes) {
    throw new Error('Not implemented');
  }

  /**
   * Get transaction by ID
   *
   * @param {string} id
   * @return {{ confirmations: number }}
   */
  // eslint-disable-next-line no-unused-vars
  getTransaction(id) {
    throw new Error('Not implemented');
  }
}

module.exports = AbstractDataProvider;
