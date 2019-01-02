class DriveDataProvider {
  /**
   * @param {fetchDapObjects} fetchDapObjects
   * @param {fetchDapContract} fetchDapContract
   * @param {RpcClient} rpcClient
   */
  constructor(fetchDapObjects, fetchDapContract, rpcClient) {
    this.fetchDapObjectsFromDrive = fetchDapObjects;
    this.fetchDapContractFromDrive = fetchDapContract;
    this.rpcClient = rpcClient;
  }

  /**
   * Fetch Dap Contract
   *
   * @param {string} id
   * @return {DapContract|null}
   */
  async fetchDapContract(id) {
    return this.fetchDapContractFromDrive(id);
  }

  /**
   * Fetch DAP Objects
   *
   * @param {string} dapContractId
   * @param {string} type
   * @param {{ where: Object }} [options]
   * @return {DapObject[]}
   */
  async fetchDapObjects(dapContractId, type, options = {}) {
    return this.fetchDapObjectsFromDrive(dapContractId, type, options);
  }

  /**
   * Fetch transaction by ID
   *
   * @param {string} id
   * @return {{ confirmations: number }}
   */
  async fetchTransaction(id) {
    return this.rpcClient.getTransaction(id);
  }
}

module.exports = DriveDataProvider;
