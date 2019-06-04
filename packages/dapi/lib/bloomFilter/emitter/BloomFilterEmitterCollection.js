class BloomFilterEmitterCollection {
  constructor() {
    this.filters = new Set();
  }

  /**
   * Add bloom filter
   *
   * @param {BloomFilterEmitter} bloomFilterEmitter
   * @return {BloomFilterEmitterCollection}
   */
  add(bloomFilterEmitter) {
    this.filters.add(bloomFilterEmitter);

    return this;
  }

  /**
   * Remove bloom filter
   *
   * @param {BloomFilterEmitter} bloomFilterEmitter
   * @return {BloomFilterEmitterCollection}
   */
  remove(bloomFilterEmitter) {
    this.filters.delete(bloomFilterEmitter);

    return this;
  }

  /**
   * Test data against bloom filters
   *
   * @param {*} data
   * @return {BloomFilterEmitterCollection}
   */
  test(data) {
    this.filters.forEach((filter) => {
      filter.test(data);
    });
  }

  /**
   * Emit event on all bloom filters
   *
   * @param {string} event
   * @param {*} data
   */
  emit(event, data) {
    this.filters.forEach((filter) => {
      filter.emit(event, data);
    });
  }
}

module.exports = BloomFilterEmitterCollection;
