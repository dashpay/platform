const pino = require('pino');
const { createHash } = require('crypto');

const logger = pino({
  prettyPrint: true,
}, 'grovedb.log');

class GroveDBStore {
  /**
   * @param {Drive} rsDrive
   * @param {string} [name]
   */
  constructor(rsDrive, name = undefined) {
    this.rsDrive = rsDrive;
    this.db = rsDrive.getGroveDB();
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

    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
      key: key.toString('hex'),
      valueHash: createHash('sha256')
        .update(value)
        .digest('hex'),
      useTransaction: Boolean(options.useTransaction),
      type: 'item',
      method,
    }, 'put');

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

    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
      key: key.toString('hex'),
      valueHash: createHash('sha256')
        .update(
          referencePath.reduce((segment, buffer) => (
            Buffer.concat([segment, buffer])
          ), Buffer.alloc(0)),
        )
        .digest('hex'),
      useTransaction: Boolean(options.useTransaction),
      type: 'reference',
      method,
    }, 'putReference');

    await this.db[method](
      path,
      key,
      { type: 'reference', value: referencePath },
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

    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
      key: key.toString('hex'),
      valueHash: createHash('sha256')
        .update(Buffer.alloc(32))
        .digest('hex'),
      useTransaction: Boolean(options.useTransaction),
      type: 'tree',
      method,
    }, 'createTree');

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
      ({ type, value } = await this.db.get(
        path,
        key,
        options.useTransaction || false,
      ));
    } catch (e) {
      if (e.message.startsWith('invalid path key: key not found in Merk')) {
        return null;
      }

      throw e;
    }

    if (type === undefined) {
      return null;
    }

    if (type !== 'item') {
      throw new Error('Key should point to item element type');
    }

    return value;
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
  async delete(path, key, options = {}) {
    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
      key: key.toString('hex'),
      useTransaction: Boolean(options.useTransaction),
      method: 'delete',
    }, 'delete');

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
  async getAux(key, options = {}) {
    let result;
    try {
      result = await this.db.getAux(
        key,
        options.useTransaction || false,
      );
    } catch (e) {
      if (e.message.startsWith('invalid path key: key not found in Merk')) {
        return null;
      }

      throw e;
    }

    return result;
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
  async putAux(key, value, options = {}) {
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
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {Promise<GroveDBStore>}
   */
  async deleteAux(key, options = {}) {
    await this.db.deleteAux(
      key,
      options.useTransaction || false,
    );

    return this;
  }

  /**
   * Get tree root hash
   *
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @return {Buffer}
   */
  async getRootHash(options = {}) {
    return this.db.getRootHash(options.useTransaction || false);
  }

  /**
   * @return {Promise<void>}
   */
  async startTransaction() {
    return this.db.startTransaction();
  }

  /**
   * @return {Promise<void>}
   */
  async isTransactionStarted() {
    return this.db.isTransactionStarted();
  }

  /**
   * Rollback transaction to this initial state when it was created
   *
   * @returns {Promise<void>}
   */
  async rollbackTransaction() {
    return this.db.rollbackTransaction();
  }

  /**
   * @return {Promise<void>}
   */
  async commitTransaction() {
    return this.db.commitTransaction();
  }

  /**
   * @return {Promise<void>}
   */
  async abortTransaction() {
    return this.db.abortTransaction();
  }

  /**
   * @return {Drive}
   */
  getDrive() {
    return this.rsDrive;
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
