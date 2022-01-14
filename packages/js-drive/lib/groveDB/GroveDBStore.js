const GroveDBTransaction = require('./GroveDBTransaction');

class GroveDBStore {
  /**
   * @param {GroveDB} db
   * @param {string} [name]
   */
  constructor(db, name = undefined) {
    this.db = db;
    this.name = name;
  }

  /**
   * Store a key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.skipIfExists]
   * @return {Promise<GroveDBStore>}
   */
  async put(path, key, value, options = {}) {
    const method = options.skipIfExists ? 'insert_if_not_exists' : 'insert';

    await this.db[method](
      path,
      key,
      { type: 'item', value },
      options.transaction,
    );

    return this;
  }

  /**
   * Store a reference to the specified key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Buffer[]} referencePath
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.skipIfExists]
   * @return {Promise<GroveDBStore>}
   */
  async putReference(path, key, referencePath, options = {}) {
    const method = options.skipIfExists ? 'insert_if_not_exists' : 'insert';

    await this.db[method](
      path,
      key,
      { type: 'reference', referencePath },
      options.transaction,
    );

    return this;
  }

  /**
   * Create empty key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.skipIfExists]
   * @return {Promise<GroveDBStore>}
   */
  async createTree(path, key, options = { }) {
    const method = options.skipIfExists ? 'insert_if_not_exists' : 'insert';

    await this.db[method](
      path,
      key,
      { type: 'tree', value: undefined },
      options.transaction,
    );

    return this;
  }

  /**
   * Get a value by key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Buffer|null}
   */
  async get(path, key, options = { }) {
    try {
      return this.db.get(
        path,
        key,
        options,
      );
    } catch (e) {
      if (e.message.indexOf('no value found for key') !== -1 || e.message.indexOf('key not found') !== -1) {
        return null;
      }

      throw e;
    }
  }

  async getWithQuery(queryPath, options) {

  }

  /**
   * Delete value by key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<GroveDBStore>}
   */
  async delete(path, key, options = { }) {
    await this.db.delete(path, key, options.transaction);

    return this;
  }

  /**
   * Fetch auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<GroveDBStore>}
   */
  async getAux(key, options) {
    await this.db.getAux(key, options.transaction);

    return this;
  }

  /**
   * Store auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<GroveDBStore>}
   */
  async putAux(key, options) {
    await this.db.putAux(key, value, options.transaction);

    return this;
  }

  /**
   * Delete auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<GroveDBStore>}
   */
  async deleteAux(key, value, options) {
    await this.db.deleteAux(key, value, options.transaction);

    return this;
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
   * @return {GroveDBTransaction}
   */
  createTransaction() {
    return new GroveDBTransaction(this.db);
  }

  /**
   * Get proof for array of keys
   *
   * @param {PathQuery} query
   * @return {Buffer}
   */
  getProve(query) {
    return this.db.proof(query);
  }

  /**
   * @returns {GroveDB}
   */
  getDB() {
    return this.db;
  }

  /**
   * @param {GroveDB} db
   */
  setDB(db) {
    this.db = db;
  }
}

module.exports = GroveDBStore;
