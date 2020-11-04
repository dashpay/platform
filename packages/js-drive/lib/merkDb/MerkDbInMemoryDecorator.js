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
    const keyString = key.toString(MerkDbInMemoryDecorator.KEY_ENCODING);

    if (this.deleted.has(keyString)) {
      throw new Error('key not found');
    }

    const value = this.data.get(keyString);
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
    const keyString = key.toString(MerkDbInMemoryDecorator.KEY_ENCODING);

    this.deleted.delete(keyString);

    this.data.set(keyString, value);

    return this;
  }

  /**
   *
   * @param {Buffer} key
   *
   * @return {MerkDbInMemoryDecorator}
   */
  delete(key) {
    const keyString = key.toString(MerkDbInMemoryDecorator.KEY_ENCODING);

    try {
      this.db.getSync(key);

      this.data.delete(keyString);
      this.deleted.add(keyString);
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
    if (this.data.size === 0 && this.deleted.size === 0) {
      // nothing to commit

      return this;
    }

    let batch = this.db.batch();

    // store values
    for (const [keyString, value] of this.data) {
      const keyBuffer = Buffer.from(keyString, MerkDbInMemoryDecorator.KEY_ENCODING);

      batch = batch.put(keyBuffer, value);
    }

    // remove keys
    for (const keyString of this.deleted) {
      const keyBuffer = Buffer.from(keyString, MerkDbInMemoryDecorator.KEY_ENCODING);

      batch = batch.delete(keyBuffer);
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

MerkDbInMemoryDecorator.KEY_ENCODING = 'hex';

module.exports = MerkDbInMemoryDecorator;
