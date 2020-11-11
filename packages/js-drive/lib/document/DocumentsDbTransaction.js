const DocumentsDBTransactionIsAlreadyStartedError = require('./errors/DocumentsDBTransactionIsAlreadyStartedError');
const DocumentsDBTransactionIsNotStartedError = require('./errors/DocumentsDBTransactionIsNotStartedError');

class DocumentsDbTransaction {
  /**
   * @param {MerkDbTransaction} documentsStoreTransaction
   * @param {MongoDBTransaction} documentMongoDBTransaction
   */
  constructor(documentsStoreTransaction, documentMongoDBTransaction) {
    this.storeTransaction = documentsStoreTransaction;
    this.mongoDbTransaction = documentMongoDBTransaction;

    this.transactionIsStarted = false;
  }

  /**
   * Get document store transaction
   *
   * @return {MerkDbTransaction}
   */
  getStoreTransaction() {
    return this.storeTransaction;
  }

  /**
   * Get MongoDb transaction
   *
   * @return {MongoDBTransaction}
   */
  getMongoDbTransaction() {
    return this.mongoDbTransaction;
  }

  /**
   * Start new transaction
   *
   * @return {Promise<void>}
   */
  async start() {
    if (this.isStarted()) {
      throw new DocumentsDBTransactionIsAlreadyStartedError();
    }

    await this.storeTransaction.start();
    await this.mongoDbTransaction.start();

    this.transactionIsStarted = true;
  }

  /**
   * Commit transaction
   *
   * @return {Promise<void>}
   */
  async commit() {
    if (!this.isStarted()) {
      throw new DocumentsDBTransactionIsNotStartedError();
    }

    await this.mongoDbTransaction.commit();
    await this.storeTransaction.commit();

    this.transactionIsStarted = false;
  }

  /**
   * Abort current transaction
   *
   * @return {Promise<void>}
   */
  async abort() {
    if (!this.isStarted()) {
      throw new DocumentsDBTransactionIsNotStartedError();
    }

    await this.mongoDbTransaction.abort();
    await this.storeTransaction.abort();

    this.transactionIsStarted = false;
  }

  /**
   * Determine if transaction is currently in progress
   *
   * @return {boolean}
   */
  isStarted() {
    return this.transactionIsStarted;
  }
}

module.exports = DocumentsDbTransaction;
