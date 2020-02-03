const { Transaction } = require('@dashevo/dashcore-lib');

class DriveDataProvider {
  /**
   * @param {fetchDocuments} fetchDocuments
   * @param {Function} fetchContract
   * @param {RpcClient} coreRPCClient
   * @param {JaysonClient} tendermintRPCClient
   * @param {MongoDBTransaction} stateViewTransaction
   * @param {DashPlatformProtocol} dpp
   */
  constructor(
    fetchDocuments,
    fetchContract,
    coreRPCClient,
    tendermintRPCClient,
    stateViewTransaction,
    dpp,
  ) {
    this.fetchDocumentsFromDrive = fetchDocuments;
    this.fetchContractFromDrive = fetchContract;
    this.coreRPCClient = coreRPCClient;
    this.tendermintRPCClient = tendermintRPCClient;
    this.stateViewTransaction = stateViewTransaction;
    this.dpp = dpp;
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
      const { result: transaction } = await this.coreRPCClient.getRawTransaction(id);
      return new Transaction(transaction);
    } catch (e) {
      // Invalid address or key error
      if (e.code === -5) {
        return null;
      }

      throw e;
    }
  }

  /**
   * Fetch identity by it's id
   *
   * @param {string} id
   *
   * @return {Promise<Identity|null>}
   */
  async fetchIdentity(id) {
    const data = Buffer.from(id).toString('hex');

    const {
      result: {
        response: {
          value: serializedIdentity,
        },
      },
    } = await this.tendermintRPCClient.request(
      'abci_query',
      {
        path: '/identity',
        data,
      },
    );

    if (!serializedIdentity) {
      return null;
    }

    return this.dpp.identity.createFromSerialized(
      Buffer.from(serializedIdentity, 'base64'),
      { skipValidation: true },
    );
  }
}

module.exports = DriveDataProvider;
