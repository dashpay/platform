class BlockExecutionDBTransactions {
  /**
   *
   * @param {MerkDbTransaction} identitiesTransaction
   * @param {MongoDBTransaction} documentTransaction
   * @param {MerkDbTransaction} dataContractsTransaction
   */
  constructor(identitiesTransaction, documentTransaction, dataContractsTransaction) {
    this.transactions = {
      identity: identitiesTransaction,
      document: documentTransaction,
      dataContract: dataContractsTransaction,
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
