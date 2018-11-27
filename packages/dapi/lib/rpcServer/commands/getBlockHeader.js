const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlockHeader');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {getBlockHeader}
 */
const getBlockHeaderFactory = (coreAPI) => {
  /**
   * Returns block headers
   * @typedef getBlockHeader
   * @param args - command arguments
   * @param {string} args.blockHash
   * @return {Promise<Array<Object>>}
   */
  async function getBlockHeader(args) {
    validator.validate(args);
    const { blockHash } = args;
    return coreAPI.getBlockHeader(blockHash);
  }

  return getBlockHeader;
};

module.exports = getBlockHeaderFactory;
