const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getAddressUnconfirmedBalance}
 */
const getAddressUnconfirmedBalanceFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns total unconfirmed balance for the address
   * @typedef getAddressUnconfirmedBalance
   * @param args - command arguments
   * @param {string} args.address
   * @return {Promise<number>}
   */
  async function getAddressUnconfirmedBalance(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getAddressUnconfirmedBalance(address);
  }

  return getAddressUnconfirmedBalance;
};

module.exports = getAddressUnconfirmedBalanceFactory;
