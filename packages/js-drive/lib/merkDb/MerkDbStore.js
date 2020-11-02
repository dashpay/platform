const MerkDbTransaction = require('./MerkDbTransaction');

class MerkDbStore {
  /**
   * @param {Merk} db
   */
  constructor(db) {
    this.db = db;
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
   * Get tree root hash
   *
   * @return {Buffer}
   */
  getRootHash() {
    return this.db.rootHash();
  }

  /**
   * Creates new transaction instance
   *
   * @return {MerkDbTransaction}
   */
  createTransaction() {
    return new MerkDbTransaction(this.db);
  }
}

module.exports = MerkDbStore;
