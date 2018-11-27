const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getAddressTotalSent}
 */
const getAddressTotalSentFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns total amount of duffs sent by the address
   * @typedef getAddressTotalSent
   * @param args - command arguments
   * @param {string} args.address
   * @return {Promise<number>}
   */
  async function getAddressTotalSent(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getAddressTotalSent(address);
  }

  return getAddressTotalSent;
};

module.exports = getAddressTotalSentFactory;
