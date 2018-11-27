const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlockHash');

const validator = new Validator(argsSchema);

/**
 * @param {Object} coreAPI
 * @return {getBlockHash}
 */
const getBlockHashFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns block hash for the given height
   * @typedef getBlockHash
   * @param args
   * @param {number} args.height - block height
   * @return {Promise<string>} - block hash
   */
  async function getBlockHash(args) {
    validator.validate(args);
    const { height } = args;
    return coreAPI.getBlockHash(height);
  }

  return getBlockHash;
};

module.exports = getBlockHashFactory;
