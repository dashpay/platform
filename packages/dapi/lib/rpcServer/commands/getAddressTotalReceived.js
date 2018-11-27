const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getAddressTotalReceived}
 */
const getAddressTotalReceivedFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns total amount of duffs received by address
   * @typedef getAddressTotalReceived
   * @param args - command arguments
   * @param {string} args.address
   * @return {Promise<number>}
   */
  async function getAddressTotalReceived(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getAddressTotalReceived(address);
  }

  return getAddressTotalReceived;
};

module.exports = getAddressTotalReceivedFactory;
