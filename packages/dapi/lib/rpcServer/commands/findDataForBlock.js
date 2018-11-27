const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getSpvData');
const BloomFilter = require('bloom-filter');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {findSpvData}
 */
const findDataForBlockFactory = (spvService) => {
  /**
   * Returns block headers
   * @typedef findDataForBlock
   * @param args - command arguments
   * @param {string} args.filter
   * @return {object}
   */
  async function findDataForBlock(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.findDataForBlock(filter, args.blockHash);
  }

  return findDataForBlock;
};

module.exports = findDataForBlockFactory;
