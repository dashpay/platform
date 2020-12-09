const MerkDbInMemoryDecorator = require('./MerkDbInMemoryDecorator');

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
  async start() {
    if (this.db) {
      throw new MerkDBTransactionIsAlreadyStartedError();
    }

    this.db = new MerkDbInMemoryDecorator(this.merkDB);

    return this;
  }

  /**
   * Commit transaction to merk DB
   *
   * @return {MerkDbTransaction}
   */
  async commit() {
    if (!this.db) {
      throw new MerkDBTransactionIsNotStartedError();
    }

    this.db.persist();

    this.db = null;

    return this;
  }

  /**
   * Abort transaction
   *
   * @return {MerkDbTransaction}
   */
  async abort() {
    if (!this.db) {
      throw new MerkDBTransactionIsNotStartedError();
    }

    this.db.reset();

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

  /**
   * Return transaction as plain object
   *
   * @return {RawStoreTransaction}
   */
  toObject() {
    if (!this.db) {
      throw new MerkDBTransactionIsNotStartedError();
    }

    const updates = {};
    for (const [key, value] of this.db.data) {
      updates[key] = value;
    }

    const deletes = {};
    for (const [key, value] of this.db.deleted) {
      deletes[key] = value;
    }

    return {
      updates,
      deletes,
    };
  }

  /**
   * Populate update and delete operations from transaction object
   *
   * @param {RawStoreTransaction} transactionObject
   *
   * @return {void}
   */
  async populateFromObject(transactionObject) {
    if (!this.isStarted()) {
      throw new MerkDBTransactionIsNotStartedError();
    }

    Object.entries(transactionObject.updates)
      .forEach(([key, value]) => this.db.data.set(key, value));

    Object.entries(transactionObject.deletes)
      .forEach(([key, value]) => this.db.deleted.set(key, value));
  }
}

module.exports = MerkDbTransaction;
