const GroveDBInMemoryDecorator = require('./GroveDBInMemoryDecorator');

const GroveDBTransactionIsNotStartedError = require('./errors/GroveDBTransactionIsNotStartedError');
const GroveDBTransactionIsAlreadyStartedError = require('./errors/GroveDBTransactionIsAlreadyStartedError');

class GroveDBTransaction {
  /**
   *
   * @param {GroveDB} groveDB
   */
  constructor(groveDB) {
    this.groveDB = groveDB;
    this.db = null;
  }

  /**
   * Start new transaction in GroveDB
   *
   * @return {GroveDBTransaction}
   */
  async start() {
    if (this.db) {
      throw new GroveDBTransactionIsAlreadyStartedError();
    }

    this.db = new GroveDBInMemoryDecorator(this.groveDB);

    return this;
  }

  /**
   * Commit transaction to merk DB
   *
   * @return {GroveDBTransaction}
   */
  async commit() {
    if (!this.db) {
      throw new GroveDBTransactionIsNotStartedError();
    }

    this.db.persist();

    this.db = null;

    return this;
  }

  /**
   * Abort transaction
   *
   * @return {GroveDBTransaction}
   */
  async abort() {
    if (!this.db) {
      throw new GroveDBTransactionIsNotStartedError();
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
      throw new GroveDBTransactionIsNotStartedError();
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
      throw new GroveDBTransactionIsNotStartedError();
    }

    Object.entries(transactionObject.updates)
      .forEach(([key, value]) => this.db.data.set(key, value));

    Object.entries(transactionObject.deletes)
      .forEach(([key, value]) => this.db.deleted.set(key, value));
  }
}

module.exports = GroveDBTransaction;
