const { promisify } = require('util');

const levelDBTransaction = require('level-transactions');

const LevelDBTransactionIsNotStartedError = require('./errors/LevelDBTransactionIsNotStartedError');
const LevelDBTransactionIsAlreadyStartedError = require('./errors/LevelDBTransactionIsAlreadyStartedError');

class LevelDBTransaction {
  /**
   * @param {LevelUP} levelDB
   * @param levelDB
   */
  constructor(levelDB) {
    this.levelDB = levelDB;
    this.db = null;
  }

  /**
   * Start new transaction in level DB
   *
   * @return {LevelDBTransaction}
   */
  start() {
    if (this.db) {
      throw new LevelDBTransactionIsAlreadyStartedError();
    }

    this.db = levelDBTransaction(this.levelDB);

    // promisify methods
    this.db.commit = promisify(this.db.commit.bind(this.db));
    this.db.rollback = promisify(this.db.rollback.bind(this.db));
    this.db.get = promisify(this.db.get.bind(this.db));
    this.db.put = promisify(this.db.put.bind(this.db));

    return this;
  }

  /**
   * Commit transaction to level DB
   *
   * @return {Promise<LevelDBTransaction>}
   */
  async commit() {
    if (!this.db) {
      throw new LevelDBTransactionIsNotStartedError();
    }

    await this.db.commit();

    this.db = null;

    return this;
  }

  /**
   * Abort transaction
   *
   * @return {Promise<LevelDBTransaction>}
   */
  async abort() {
    if (!this.db) {
      throw new LevelDBTransactionIsNotStartedError();
    }

    await this.db.rollback();

    this.db = null;

    return this;
  }
}

module.exports = LevelDBTransaction;
