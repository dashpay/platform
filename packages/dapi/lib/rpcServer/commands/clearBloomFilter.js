const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/clearBloomFilter');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {clearBloomFilter}
 */
const clearBloomFilterFactory = (spvService) => {
  /**
   * Layer 1 endpoint
   * clears bloom filter
   * @typedef clearBloomFilter
   * @param args - command arguments
   * @param {string} args.filter
   * @return {Promise<bool>}
   */
  async function clearBloomFilter(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.clearBloomFilter(filter);
  }

  return clearBloomFilter;
};

module.exports = clearBloomFilterFactory;
