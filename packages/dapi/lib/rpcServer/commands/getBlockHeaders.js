const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlockHeaders');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {getBlockHeaders}
 */
const getBlockHeadersFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns block headers
   * @typedef getBlockHeaders
   * @param args - command arguments
   * @param {number} args.offset - block height starting point
   * @param {number} args.limit - number of block headers to return
   * @return {Promise<Array<Object>>}
   */
  async function getBlockHeaders(args) {
    validator.validate(args);
    const { offset, limit } = args;
    return coreAPI.getBlockHeaders(typeof offset === 'number' ? await coreAPI.getBlockHash(offset) : offset, limit);
  }

  return getBlockHeaders;
};

module.exports = getBlockHeadersFactory;
