const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getMnListDiff');

const validator = new Validator(argsSchema);
/**
 * Returns getAddressTotalReceived function
 * @param coreAPI
 * @return {getMnListDiff}
 */
const getMnListDiffFactory = (coreAPI) => {
  /**
   * Returns calculated balance for the address
   * @typedef getMnListDiff
   * @param args - command arguments
   * @param baseBlockHash {string}
   * @param blockHash {string}
   * @return {Promise<string>}
   */
  async function getMnListDiff(args) {
    validator.validate(args);
    const { baseBlockHash, blockHash } = args;
    return coreAPI.getMnListDiff(baseBlockHash, blockHash);
  }

  return getMnListDiff;
};

module.exports = getMnListDiffFactory;
