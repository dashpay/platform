const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);
/**
 * Returns getAddressTotalReceived function
 * @param coreAPI
 * @return {getBalance}
 */
const getBalanceFactory = (coreAPI) => {
  /**
   * Returns calculated balance for the address
   * @typedef getBalance
   * @param args - command arguments
   * @param {string} args.address
   * @return {Promise<number>}
   */
  async function getBalance(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getBalance(address);
  }

  return getBalance;
};

module.exports = getBalanceFactory;
