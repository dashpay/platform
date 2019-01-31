class DriveDataProvider {
  /**
   * @param {fetchDPObjects} fetchDPObjects
   * @param {Function} createFetchDPContract
   * @param {RpcClient} rpcClient
   */
  constructor(fetchDPObjects, createFetchDPContract, rpcClient) {
    this.fetchDPObjectsFromDrive = fetchDPObjects;
    this.createFetchDPContract = createFetchDPContract;
    this.rpcClient = rpcClient;
  }

  /**
   * Fetch DP Contract by ID
   *
   * @param {string} id
   * @returns {Promise<DPContract|null>}
   */
  async fetchDPContract(id) {
    return this.createFetchDPContract()(id);
  }

  /**
   * Fetch DP Objects by contract ID and type
   *
   * @param {string} dpContractId
   * @param {string} type
   * @param {{ where: Object }} [options]
   * @returns {Promise<DPObject[]>}
   */
  async fetchDPObjects(dpContractId, type, options = {}) {
    return this.fetchDPObjectsFromDrive(dpContractId, type, options);
  }

  /**
   * Fetch transaction by ID
   *
   * @param {string} id
   * @returns {Promise<{ confirmations: number }|null>}
   */
  async fetchTransaction(id) {
    try {
      return await this.rpcClient.getRawTransaction(id);
    } catch (e) {
      // Invalid address or key error
      if (e.code === -5) {
        return null;
      }

      throw e;
    }
  }
}

module.exports = DriveDataProvider;
