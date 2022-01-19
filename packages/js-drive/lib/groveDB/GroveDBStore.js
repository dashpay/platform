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
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   * @return {Promise<GroveDBStore>}
   */
  async put(path, key, value, options = {}) {
    const method = options.skipIfExists ? 'insertIfNotExists' : 'insert';

    await this.db[method](
      path,
      key,
      { type: 'item', value },
      options.useTransaction || false,
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
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   * @return {Promise<GroveDBStore>}
   */
  async putReference(path, key, referencePath, options = {}) {
    const method = options.skipIfExists ? 'insertIfNotExists' : 'insert';

    await this.db[method](
      path,
      key,
      { type: 'reference', referencePath },
      options.useTransaction || false,
    );

    return this;
  }

  /**
   * Create empty key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   * @return {Promise<GroveDBStore>}
   */
  async createTree(path, key, options = { }) {
    const method = options.skipIfExists ? 'insertIfNotExists' : 'insert';

    await this.db[method](
      path,
      key,
      { type: 'tree', value: Buffer.alloc(32) },
      options.useTransaction || false,
    );

    return this;
  }

  /**
   * Get a value by key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {Buffer|null}
   */
  async get(path, key, options = { }) {
    let type;
    let value;

    try {
      ({ type, value } = this.db.get(
        path,
        key,
        options.useTransaction || false,
      ));
    } catch (e) {
      if (e.message === 'invalid path: key not found in Merk') {
        return null;
      }

      throw e;
    }

    if (type !== 'item') {
      throw new Error('Key should point to item element type');
    }

    return value;
  }

  async getWithQuery(queryPath, options) {

  }

  /**
   * Delete value by key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {Promise<GroveDBStore>}
   */
  async delete(path, key, options = { }) {
    await this.db.delete(
      path,
      key,
      options.useTransaction || false,
    );

    return this;
  }

  /**
   * Get auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {Promise<GroveDBStore>}
   */
  async getAux(key, options) {
    return this.db.getAux(
      key,
      options.useTransaction || false,
    );
  }

  /**
   * Store auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {Promise<GroveDBStore>}
   */
  async putAux(key, value, options) {
    await this.db.putAux(
      key,
      value,
      options.useTransaction || false,
    );

    return this;
  }

  /**
   * Delete auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {Promise<GroveDBStore>}
   */
  async deleteAux(key, value, options) {
    await this.db.deleteAux(
      key,
      value,
      options.useTransaction || false,
    );

    return this;
  }

  /**
   * Get tree root hash
   *
   * @return {Buffer}
   */
  async getRootHash() {
    return Buffer.from(await this.db.rootHash());
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
