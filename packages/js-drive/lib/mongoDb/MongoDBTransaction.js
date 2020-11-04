const TransactionIsNotStartedError = require('./errors/TransactionIsNotStartedError');
const TransactionIsAlreadyStartedError = require('./errors/TransactionIsAlreadyStartedError');

class MongoDBTransaction {
  /**
   * @param {connectToMongoDB} connectToDocumentMongoDB
   */
  constructor(connectToDocumentMongoDB) {
    this.connectToDocumentMongoDB = connectToDocumentMongoDB;
    this.session = null;
    this.mongoClient = null;
    this.isTransactionStarted = false;
  }

  /**
   * Start new transaction
   */
  async start() {
    if (this.isTransactionStarted) {
      throw new TransactionIsAlreadyStartedError();
    }

    this.mongoClient = await this.connectToDocumentMongoDB();

    if (!this.session || this.session.hasEnded) {
      this.session = this.mongoClient.startSession();
    }

    this.session.startTransaction();
    this.isTransactionStarted = true;
  }

  /**
   * Commit transaction
   *
   * @returns {Promise<void>}
   */
  async commit() {
    if (!this.isTransactionStarted) {
      throw new TransactionIsNotStartedError();
    }

    const { ERRORS } = MongoDBTransaction;

    try {
      await this.session.commitTransaction();

      this.isTransactionStarted = false;
    } catch (error) {
      if (error.errorLabels
        && error.errorLabels.indexOf(ERRORS.UNKNOWN_TRANSACTION_COMMIT_RESULT) >= 0) {
        await this.commit();
      } else {
        throw error;
      }
    }
  }

  /**
   * Abort current transaction
   *
   * @returns {Promise<void>}
   */
  async abort() {
    if (!this.isTransactionStarted) {
      throw new TransactionIsNotStartedError();
    }

    await this.session.abortTransaction();
    this.isTransactionStarted = false;
  }

  /**
   * Determine if transaction is currently in progress
   *
   * @return {boolean}
   */
  isStarted() {
    return this.isTransactionStarted;
  }

  /**
   * Run query to mongoDB with transaction
   *
   * @param {Function} transactionFunction
   * @returns {Promise<*>}
   */
  async runWithTransaction(transactionFunction) {
    if (!this.isTransactionStarted) {
      throw new TransactionIsNotStartedError();
    }

    const { ERRORS } = MongoDBTransaction;

    try {
      return await transactionFunction(this.mongoClient, this.session);
    } catch (error) {
      if (error.errorLabels
        && error.errorLabels.indexOf(ERRORS.TRANSIENT_TRANSACTION_ERROR) >= 0) {
        return this.runWithTransaction(transactionFunction);
      }

      throw error;
    }
  }
}

MongoDBTransaction.ERRORS = {
  UNKNOWN_TRANSACTION_COMMIT_RESULT: 'UnknownTransactionCommitResult',
  TRANSIENT_TRANSACTION_ERROR: 'TransientTransactionError',
};

module.exports = MongoDBTransaction;
