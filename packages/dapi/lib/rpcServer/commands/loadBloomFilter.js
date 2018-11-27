const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/loadBloomFilter');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {loadBloomFilter}
 */
const loadBloomFilterFactory = (spvService) => {
  /**
   * Layer 1 endpoint
   * Returns boolean
   * @typedef loadBloomFilter
   * @param args - command arguments
   * @param {string} args.filter
   * @return {Promise<bool>}
   */
  async function loadBloomFilter(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.loadBloomFilter(filter);
  }

  return loadBloomFilter;
};

module.exports = loadBloomFilterFactory;
