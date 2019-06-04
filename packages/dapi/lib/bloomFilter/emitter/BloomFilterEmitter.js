const { EventEmitter } = require('events');

class BloomFilterEmitter extends EventEmitter {
  /**
   * @param {Filter} bloomFilter
   * @param {testFunction} testFunction
   */
  constructor(bloomFilter, testFunction) {
    super();

    this.bloomFilter = bloomFilter;
    this.testFunction = testFunction;
  }

  /**
   * Test data against bloom filter
   *
   * @param {*} data
   * @return {boolean}
   */
  test(data) {
    const result = this.testFunction(this.bloomFilter, data);

    if (result) {
      this.emit('match', data);
    }

    return result;
  }
}

/**
 * @typedef testFunction
 * @param {Filter} filter
 * @param {*} data
 */

module.exports = BloomFilterEmitter;
