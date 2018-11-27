const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getSpvData');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {getSpvData}
 */
const getSpvDataFactory = (spvService) => {
  /**
   * Returns block headers
   * @typedef getSpvData
   * @param args - command arguments
   * @param {string} args.filter
   * @return {object}
   */
  async function getSpvData(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.getSpvData(filter);
  }

  return getSpvData;
};

module.exports = getSpvDataFactory;
