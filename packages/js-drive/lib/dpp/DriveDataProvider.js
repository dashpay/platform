class DriveDataProvider {
  /**
   * @param {fetchDocuments} fetchDocuments
   * @param {Function} createFetchContract
   * @param {RpcClient} rpcClient
   */
  constructor(fetchDocuments, createFetchContract, rpcClient) {
    this.fetchDocumentsFromDrive = fetchDocuments;
    this.createFetchContract = createFetchContract;
    this.rpcClient = rpcClient;
  }

  /**
   * Fetch Contract by ID
   *
   * @param {string} id
   * @returns {Promise<Contract|null>}
   */
  async fetchContract(id) {
    return this.createFetchContract()(id);
  }

  /**
   * Fetch Documents by contract ID and type
   *
   * @param {string} contractId
   * @param {string} type
   * @param {{ where: Object }} [options]
   * @returns {Promise<Document[]>}
   */
  async fetchDocuments(contractId, type, options = {}) {
    return this.fetchDocumentsFromDrive(contractId, type, options);
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
