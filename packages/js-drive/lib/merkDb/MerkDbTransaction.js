const MerkDbTransactionDecorator = require('./MerkDbInMemoryDecorator');

const MerkDBTransactionIsNotStartedError = require('./errors/MerkDBTransactionIsNotStartedError');
const MerkDBTransactionIsAlreadyStartedError = require('./errors/MerkDBTransactionIsAlreadyStartedError');

class MerkDbTransaction {
  /**
   *
   * @param {Merk} merkDB
   */
  constructor(merkDB) {
    this.merkDB = merkDB;
    this.db = null;
  }

  /**
   * Start new transaction in merk DB
   *
   * @return {MerkDbTransaction}
   */
  start() {
    if (this.db) {
      throw new MerkDBTransactionIsAlreadyStartedError();
    }

    this.db = new MerkDbTransactionDecorator(this.merkDB);

    return this;
  }

  /**
   * Commit transaction to merk DB
   *
   * @return {MerkDbTransaction}
   */
  commit() {
    if (!this.db) {
      throw new MerkDBTransactionIsNotStartedError();
    }

    this.db.commit();

    this.db = null;

    return this;
  }

  /**
   * Abort transaction
   *
   * @return {MerkDbTransaction}
   */
  abort() {
    if (!this.db) {
      throw new MerkDBTransactionIsNotStartedError();
    }

    this.db.rollback();

    this.db = null;

    return this;
  }

  /**
   * Determine if transaction is currently in progress
   *
   * @return {boolean}
   */
  isStarted() {
    return this.db !== null;
  }
}

module.exports = MerkDbTransaction;
