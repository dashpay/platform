class BlockExecutionDBTransactions {
  /**
   *
   * @param {LevelDBTransaction} identityTransaction
   * @param {MongoDBTransaction} documentTransaction
   * @param {LevelDBTransaction} dataContractTransaction
   */
  constructor(identityTransaction, documentTransaction, dataContractTransaction) {
    this.transactions = {
      identity: identityTransaction,
      document: documentTransaction,
      dataContract: dataContractTransaction,
    };
  }

  /**
   * Start transactions
   */
  start() {
    Object.values(this.transactions).map((t) => t.start());
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
   * @return {LevelDBTransaction|MongoDBTransaction}
   */
  getTransaction(name) {
    return this.transactions[name];
  }
}

module.exports = BlockExecutionDBTransactions;
