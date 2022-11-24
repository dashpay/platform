const { createHash } = require('crypto');
const StorageResult = require('./StorageResult');

class GroveDBStore {
  /**
   * @param {Drive} rsDrive
   * @param {Object} [logger]
   */
  constructor(rsDrive, logger = undefined) {
    this.rsDrive = rsDrive;
    this.db = rsDrive.getGroveDB();
    this.logger = logger;
  }

  /**
   * Store a key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.skipIfExists=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async put(path, key, value, options = {}) {
    const method = options.skipIfExists ? 'insertIfNotExists' : 'insert';

    try {
      await this.db[method](
        path,
        key,
        {
          type: 'item',
          value,
        },
        options.transaction || undefined,
      );
    } finally {
      if (this.logger) {
        this.logger.info({
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
          transaction: options.transaction ? 'yes' : 'no',
          type: 'item',
          method,
          appHash: (await this.getRootHash(options)).toString('hex'),
        }, 'put');
      }
    }

    return new StorageResult(
      undefined,
      [],
    );
  }

  /**
   * Store a reference to the specified key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Buffer[]} referencePath
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.skipIfExists=false]
   * @return {Promise<StorageResult<void>>}
   */
  async putReference(path, key, referencePath, options = {}) {
    const method = options.skipIfExists ? 'insertIfNotExists' : 'insert';

    try {
      await this.db[method](
        path,
        key,
        {
          type: 'reference',
          value: {
            type: 'absolutePathReference',
            path: referencePath,
          },
        },
        options.transaction || undefined,
      );
    } finally {
      if (this.logger) {
        this.logger.info({
          path: path.map((segment) => segment.toString('hex')),
          pathHash: createHash('sha256')
            .update(
              path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
            )
            .digest('hex'),
          key: key.toString('hex'),
          value: referencePath.map((segment) => segment.toString('hex')),
          valueHash: createHash('sha256')
            .update(
              referencePath.reduce((segment, buffer) => (
                Buffer.concat([segment, buffer])
              ), Buffer.alloc(0)),
            )
            .digest('hex'),
          transaction: options.transaction,
          type: 'reference',
          method,
          appHash: (await this.getRootHash(options)).toString('hex'),
        }, 'putReference');
      }
    }

    return new StorageResult(
      undefined,
      [],
    );
  }

  /**
   * Create empty key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.skipIfExists=false]
   * @return {Promise<StorageResult<void>>}
   */
  async createTree(path, key, options = { }) {
    const method = options.skipIfExists ? 'insertIfNotExists' : 'insert';

    try {
      await this.db[method](
        path,
        key,
        {
          type: 'tree',
        },
        options.transaction || undefined,
      );
    } finally {
      if (this.logger) {
        this.logger.info({
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
          transaction: options.transaction ? 'yes' : 'no',
          type: 'tree',
          method,
          appHash: (await this.getRootHash(options)).toString('hex'),
        }, 'createTree');
      }
    }

    return new StorageResult(
      undefined,
      [],
    );
  }

  /**
   * Get a value by key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {number} [options.predictedValueSize]
   * @return {Promise<StorageResult<Buffer|null>>}
   */
  async get(path, key, options = { }) {
    let type;
    let value;

    try {
      ({
        type,
        value,
      } = await this.db.get(
        path,
        key,
        options.transaction || undefined,
      ));
    } catch (e) {
      if (
        e.message.startsWith('grovedb: path key not found')
        || e.message.startsWith('grovedb: path not found')
      ) {
        return new StorageResult(
          null,
          [],
        );
      }

      throw e;
    }

    if (type === undefined) {
      return new StorageResult(
        null,
        [],
      );
    }

    if (type !== 'item') {
      throw new Error('Key should point to item element type');
    }

    return new StorageResult(
      value,
      [],
    );
  }

  /**
   * Query keys and values
   *
   * @param {PathQuery} query
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<StorageResult<Buffer|null>>}
   */
  async query(query, options = { }) {
    let items;

    try {
      [items] = await this.db.query(
        query,
        options.transaction || undefined,
      );
    } catch (e) {
      if (
        e.message.startsWith('grovedb: path key not found')
        || e.message.startsWith('grovedb: path not found')
      ) {
        return new StorageResult(
          null,
          [],
        );
      }

      throw e;
    }

    return new StorageResult(
      items,
      [],
    );
  }

  /**
   * Prove query
   *
   * @param {PathQuery} query
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<StorageResult<Buffer>>}
   * */
  async proveQuery(query, options = {}) {
    const proof = await this.db.proveQuery(
      query,
      options.transaction || undefined,
    );

    return new StorageResult(
      proof,
      [],
    );
  }

  /**
   * Prove many queries
   *
   * @param {PathQuery[]} queries
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<StorageResult<Buffer>>}
   * */
  async proveQueryMany(queries, options = {}) {
    const proof = await this.db.proveQueryMany(
      queries,
      options.transaction || undefined,
    );

    return new StorageResult(
      proof,
      [],
    );
  }

  /**
   * Delete value by key
   *
   * @param {Buffer[]} path
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Promise<StorageResult<void>>}
   */
  async delete(path, key, options = {}) {
    try {
      await this.db.delete(
        path,
        key,
        options.transaction || undefined,
      );
    } finally {
      if (this.logger) {
        this.logger.info({
          path: path.map((segment) => segment.toString('hex')),
          pathHash: createHash('sha256')
            .update(
              path.reduce((segment, buffer) => Buffer.concat([segment, buffer]), Buffer.alloc(0)),
            ).digest('hex'),
          key: key.toString('hex'),
          transaction: options.transaction,
          method: 'delete',
          appHash: (await this.getRootHash(options)).toString('hex'),
        }, 'delete');
      }
    }

    return new StorageResult(
      undefined,
      [],
    );
  }

  /**
   * Get auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.predictedValueSize]
   * @return {Promise<StorageResult<Buffer|null>>}
   */
  async getAux(key, options = {}) {
    let result = null;

    try {
      result = await this.db.getAux(
        key,
        options.transaction || undefined,
      );
    } catch (e) {
      if (e.message.startsWith('grovedb: path key not found')) {
        return new StorageResult(
          null,
          [],
        );
      }

      throw e;
    }

    return new StorageResult(
      result,
      [],
    );
  }

  /**
   * Store auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Buffer} value
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<void>>}
   */
  async putAux(key, value, options = {}) {
    try {
      await this.db.putAux(
        key,
        value,
        options.transaction || undefined,
      );
    } finally {
      if (this.logger) {
        this.logger.info({
          key: key.toString('hex'),
          value: value.toString('hex'),
          valueHash: createHash('sha256')
            .update(value)
            .digest('hex'),
          transaction: options.transaction,
          method: 'putAux',
          appHash: (await this.getRootHash(options)).toString('hex'),
        }, 'putAux');
      }
    }

    return new StorageResult(
      undefined,
      [],
    );
  }

  /**
   * Delete auxiliary value by key
   *
   * @param {Buffer} key
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<void>>}
   */
  async deleteAux(key, options = {}) {
    try {
      await this.db.deleteAux(
        key,
        options.transaction || undefined,
      );
    } finally {
      if (this.logger) {
        this.logger.info({
          key: key.toString('hex'),
          transaction: options.transaction,
          method: 'deleteAux',
          appHash: (await this.getRootHash(options)).toString('hex'),
        }, 'deleteAux');
      }
    }

    return new StorageResult(
      undefined,
      [],
    );
  }

  /**
   * Get tree root hash
   *
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @return {Buffer}
   */
  async getRootHash(options = {}) {
    return this.db.getRootHash(options.transaction || undefined);
  }

  /**
   * @return {Promise<GroveDBTransaction>}
   */
  async startTransaction() {
    return this.db.startTransaction();
  }

  /**
   * Rollback transaction to this initial state when it was created
   *
   * @param {GroveDBTransaction} transaction
   * @returns {Promise<void>}
   */
  async rollbackTransaction(transaction) {
    return this.db.rollbackTransaction(transaction);
  }

  /**
   * @param {GroveDBTransaction} transaction
   * @return {Promise<void>}
   */
  async commitTransaction(transaction) {
    return this.db.commitTransaction(transaction);
  }

  /**
   * @param {GroveDBTransaction} transaction
   * @return {Promise<void>}
   */
  async abortTransaction(transaction) {
    return this.db.abortTransaction(transaction);
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
