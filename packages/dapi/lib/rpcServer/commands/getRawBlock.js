const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getRawBlock');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getRawBlock}
 */
const getRawBlockFactory = (coreAPI) => {
  // Todo: document summary format
  /**
   * Returns raw block for the given block hash
   * @typedef getRawBlock
   * @param args
   * @param {string} args.blockHash
   * @return {Promise<object>}
   */
  async function getRawBlock(args) {
    validator.validate(args);
    const { blockHash } = args;
    return coreAPI.getRawBlock(blockHash);
  }

  return getRawBlock;
};

module.exports = getRawBlockFactory;
