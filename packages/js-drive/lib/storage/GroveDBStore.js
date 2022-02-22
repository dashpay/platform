const { createHash } = require('crypto');

const logger = require('../util/noopLogger');

class GroveDBStore {
  /**
   * @param {Drive} rsDrive
   * @param {string} [name]
   */
  constructor(rsDrive, name = undefined) {
    this.rsDrive = rsDrive;
    this.db = rsDrive.getGroveDB();
    this.name = name;
    this.logger = logger;
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

    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: createHash('sha256')
        .update(
          path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
        ).digest('hex'),
      key: key.toString('hex'),
      value: value.toString('hex'),
      valueHash: createHash('sha256')
        .update(value)
        .digest('hex'),
      useTransaction: Boolean(options.useTransaction),
      type: 'item',
      method,
      appHash: (await this.getRootHash(options)).toString('hex'),
    }, 'put');

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
      { type: 'reference', value: referencePath },
      options.useTransaction || false,
    );

    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: createHash('sha256')
        .update(
          path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
        ).digest('hex'),
      key: key.toString('hex'),
      value: referencePath.map((segment) => segment.toString('hex')),
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
      appHash: (await this.getRootHash(options)).toString('hex'),
    }, 'putReference');

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

    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: createHash('sha256')
        .update(
          path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
        ).digest('hex'),
      key: key.toString('hex'),
      value: Buffer.alloc(32).toString('hex'),
      valueHash: createHash('sha256')
        .update(Buffer.alloc(32))
        .digest('hex'),
      useTransaction: Boolean(options.useTransaction),
      type: 'tree',
      method,
      appHash: (await this.getRootHash(options)).toString('hex'),
    }, 'createTree');

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
      if (e.message.startsWith('path key not found') || e.message.startsWith('path not found')) {
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
    await this.db.delete(
      path,
      key,
      options.useTransaction || false,
    );

    logger.info({
      path: path.map((segment) => segment.toString('hex')),
      pathHash: createHash('sha256')
        .update(
          path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
        ).digest('hex'),
      key: key.toString('hex'),
      useTransaction: Boolean(options.useTransaction),
      method: 'delete',
      appHash: (await this.getRootHash(options)).toString('hex'),
    }, 'delete');

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
      if (e.message.startsWith('path key not found')) {
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

    logger.info({
      key: key.toString('hex'),
      value: value.toString('hex'),
      valueHash: createHash('sha256')
        .update(value)
        .digest('hex'),
      useTransaction: Boolean(options.useTransaction),
      method: 'putAux',
      appHash: (await this.getRootHash(options)).toString('hex'),
    }, 'putAux');

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

    logger.info({
      key: key.toString('hex'),
      useTransaction: Boolean(options.useTransaction),
      method: 'deleteAux',
      appHash: (await this.getRootHash(options)).toString('hex'),
    }, 'deleteAux');

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
