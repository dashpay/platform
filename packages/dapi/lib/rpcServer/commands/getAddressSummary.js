const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getAddressSummary}
 */
const getAddressSummaryFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * get summary for address
   * @typedef getAddressSummary
   * @param args
   * @param {string} args.address
   * @return {Promise<object>}
   */
  async function getAddressSummary(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getAddressSummary(address);
  }

  return getAddressSummary;
};

module.exports = getAddressSummaryFactory;
