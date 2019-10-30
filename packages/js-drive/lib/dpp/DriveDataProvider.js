const { Transaction } = require('@dashevo/dashcore-lib');

class DriveDataProvider {
  /**
   * @param {fetchDocuments} fetchDocuments
   * @param {Function} fetchContract
   * @param {RpcClient} rpcClient
   * @param {MongoDBTransaction} stateViewTransaction
   */
  constructor(fetchDocuments, fetchContract, rpcClient, stateViewTransaction) {
    this.fetchDocumentsFromDrive = fetchDocuments;
    this.fetchContractFromDrive = fetchContract;
    this.rpcClient = rpcClient;
    this.stateViewTransaction = stateViewTransaction;
  }

  /**
   * Fetch Data Contract by ID
   *
   * @param {string} id
   * @returns {Promise<DataContract|null>}
   */
  async fetchDataContract(id) {
    return this.fetchContractFromDrive(id);
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
    return this.fetchDocumentsFromDrive(contractId, type, options, this.stateViewTransaction);
  }

  /**
   * Fetch transaction by ID
   *
   * @param {string} id
   * @returns {Promise<{ confirmations: number }|null>}
   */
  async fetchTransaction(id) {
    try {
      const { result: transaction } = await this.rpcClient.getRawTransaction(id);
      return new Transaction(transaction);
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
