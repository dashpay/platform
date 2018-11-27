const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getTransactionById');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getTransactionById}
 */
const getTransactionByIdFactory = (coreAPI) => {
  /**
   * Returns transaction for the given hash
   * @typedef getTransactionById
   * @param args
   * @param {string} args.txid
   * @return {Promise<object>}
   */
  async function getTransactionById(args) {
    validator.validate(args);
    const { txid } = args;
    return coreAPI.getTransactionById(txid);
  }

  return getTransactionById;
};

module.exports = getTransactionByIdFactory;
