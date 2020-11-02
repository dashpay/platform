class MerkDbInMemoryDecorator {
  /**
   * @param {Merk} merkDB
   */
  constructor(merkDB) {
    this.db = merkDB;
    this.deleted = new Set();
    this.data = new Map();
  }

  /**
   *
   * @param {Buffer} key
   * @return {null|Buffer}
   */
  getSync(key) {
    if (this.deleted[key.toString('hex')]) {
      throw new Error('key not found');
    }

    const value = this.data.get(key.toString('hex'));
    if (value !== undefined) {
      return value;
    }

    return this.db.getSync(key);
  }

  /**
   *
   * @param {Buffer} key
   * @param {*} value
   *
   * @return {MerkDbInMemoryDecorator}
   */
  put(key, value) {
    this.deleted.delete(key.toString('hex'));

    this.data.set(key.toString('hex'), value);

    return this;
  }

  /**
   *
   * @param {Buffer} key
   *
   * @return {MerkDbInMemoryDecorator}
   */
  delete(key) {
    try {
      this.db.getSync(key);

      this.data.delete(key.toString('hex'));
      this.deleted.add(key.toString('hex'));
    } catch (e) {
      if (!e.message.startsWith('key not found')) {
        throw e;
      }
    }

    return this;
  }

  /**
   * Persist in memory data to MerkDb
   *
   * @return {MerkDbInMemoryDecorator}
   */
  persist() {
    if (!this.data.size && !this.deleted.size) {
      // nothing to commit

      return this;
    }

    let batch = this.db.batch();

    // store values
    for (const [key, value] of this.data) {
      batch = batch.put(Buffer.from(key, 'hex'), value);
    }

    // remove keys
    for (const key of this.deleted) {
      batch = batch.delete(Buffer.from(key, 'hex'));
    }

    // commit
    batch.commitSync();

    // reset in memory memory
    this.reset();

    return this;
  }

  /**
   * Reset in memory data
   *
   * @return {MerkDbInMemoryDecorator}
   */
  reset() {
    this.data.clear();
    this.deleted.clear();

    return this;
  }
}

module.exports = MerkDbInMemoryDecorator;
