const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getUTXO}
 */
const getUTXOFactory = (coreAPI) => {
  /**
   * Returns unspent outputs for the given address
   * @typedef getUTXO
   * @param args
   * @param {string} args.address
   * @return {Promise<Array<Object>>}
   */
  async function getUTXO(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getUTXO(address);
  }

  return getUTXO;
};

module.exports = getUTXOFactory;
