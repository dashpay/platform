class BlockExecutionDBTransactions {
  /**
   *
   * @param {MerkDbTransaction} identitiesTransaction
   * @param {DocumentsDbTransaction} documentsDbTransaction
   * @param {MerkDbTransaction} dataContractsTransaction
   * @param {MerkDbTransaction} publicKeyToIdentityIdTransaction
   */
  constructor(
    identitiesTransaction,
    documentsDbTransaction,
    dataContractsTransaction,
    publicKeyToIdentityIdTransaction,
  ) {
    this.transactions = {
      identity: identitiesTransaction,
      document: documentsDbTransaction,
      dataContract: dataContractsTransaction,
      publicKeyToIdentityId: publicKeyToIdentityIdTransaction,
    };
  }

  /**
   * Start transactions
   */
  async start() {
    await Promise.all(
      Object.values(this.transactions).map((t) => t.start()),
    );
  }

  /**
   * Commit transactions
   *
   * @return {Promise<void>}
   */
  async commit() {
    await Promise.all(
      Object
        .values(this.transactions)
        .map((transaction) => transaction.commit()),
    );
  }

  /**
   * Abort transactions
   *
   * @return {Promise<void>}
   */
  async abort() {
    await Promise.all(
      Object
        .values(this.transactions)
        .map((transaction) => transaction.abort()),
    );
  }

  /**
   * Get transaction by name
   *
   * @return {MerkDbTransaction|MongoDBTransaction}
   */
  getTransaction(name) {
    return this.transactions[name];
  }
}

module.exports = BlockExecutionDBTransactions;
