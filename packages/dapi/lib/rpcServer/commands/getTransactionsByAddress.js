const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getTransactionsByAddress}
 */
const getTransactionsByAddressFactory = (coreAPI) => {
  /**
   * Returns all transaction related to the given address
   * @typedef getTransactionsByAddress
   * @param args
   * @param {string} args.address
   * @return {Promise<Array<Object>>}
   */
  async function getTransactionsByAddress(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getTransactionsByAddress(address);
  }

  return getTransactionsByAddress;
};

module.exports = getTransactionsByAddressFactory;
