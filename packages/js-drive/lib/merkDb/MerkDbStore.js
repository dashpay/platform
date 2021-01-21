const MerkDbTransaction = require('./MerkDbTransaction');

class MerkDbStore {
  /**
   * @param {Merk} db
   * @param {BaseLogger} [logger]
   * @param {string} [name]
   */
  constructor(db, logger = undefined, name = undefined) {
    this.db = db;
    this.logger = logger;
    this.name = name;
  }

  /**
   * Store a key into store
   *
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {MerkDbTransaction} [transaction]
   * @return {MerkDbStore}
   */
  put(key, value, transaction = undefined) {
    if (this.logger) {
      this.logger.trace({
        storeName: this.name,
        key: key.toString('hex'),
        value: value.toString('hex'),
        transaction: Boolean(transaction),
      }, `Update ${key.toString('hex')} in ${this.name} store`);
    }

    if (transaction) {
      transaction.db.put(key, value);
    } else {
      this.db
        .batch()
        .put(key, value)
        .commitSync();
    }

    return this;
  }

  /**
   * Get a value by key
   *
   * @param {Buffer} key
   * @param {MerkDbTransaction} [transaction]
   * @return {Promise<Buffer|null>}
   */
  get(key, transaction = undefined) {
    const db = transaction ? transaction.db : this.db;

    try {
      return db.getSync(key);
    } catch (e) {
      if (e.message.startsWith('key not found')) {
        return null;
      }

      throw e;
    }
  }

  /**
   * Delete value by key
   *
   * @param {Buffer} key
   * @param {MerkDbTransaction} [transaction]
   */
  delete(key, transaction = undefined) {
    if (transaction) {
      transaction.db.delete(key);
    } else {
      this.db
        .batch()
        .delete(key)
        .commitSync();
    }
  }

  /**
   * Get tree root hash
   *
   * @return {Buffer}
   */
  getRootHash() {
    return Buffer.from(this.db.rootHash());
  }

  /**
   * Creates new transaction instance
   *
   * @return {MerkDbTransaction}
   */
  createTransaction() {
    return new MerkDbTransaction(this.db);
  }

  /**
   * Get proof for array of keys
   *
   * @param {Array<Buffer>} keys
   * @return {Buffer}
   */
  getProof(keys) {
    return this.db.proveSync(keys);
  }
}

module.exports = MerkDbStore;
