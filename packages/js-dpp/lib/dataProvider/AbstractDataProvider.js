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
   * Fetch DAP Objects
   *
   * @param {string} dapContractId
   * @param {string} type
   * @param {{ where: Object }} [options]
   * @return {DapObject[]}
   */
  // eslint-disable-next-line no-unused-vars
  fetchDapObjects(dapContractId, type, options = {}) {
    throw new Error('Not implemented');
  }

  /**
   * Fetch transaction by ID
   *
   * @param {string} id
   * @return {{ confirmations: number }}
   */
  // eslint-disable-next-line no-unused-vars
  fetchTransaction(id) {
    throw new Error('Not implemented');
  }
}

module.exports = AbstractDataProvider;
