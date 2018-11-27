const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlocks');

const validator = new Validator(argsSchema);
/**
 * Returns getBlocks function
 * @param coreAPI
 * @return {getBlocks}
 */
const getBlocksFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns info for blocks
   * @typedef getBlocks
   * @param args - command arguments
   * @param {string} args.blockDate
   * @param {number} args.limit - number of blocks to return
   * @return {Promise<object[]>}
   */
  async function getBlocks(args) {
    validator.validate(args);
    const { limit, blockDate } = args;
    return coreAPI.getBlocks(blockDate, limit);
  }

  return getBlocks;
};

module.exports = getBlocksFactory;
