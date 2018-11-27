const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/sendRawIxTransaction');

const validator = new Validator(argsSchema);
/**
 * Sends raw transaction to the network
 * @param coreAPI
 * @return {sendRawIxTransaction}
 */
const sendRawIxTransactionFactory = (coreAPI) => {
  /**
   * Sends raw instant send tx and returns the txid
   * @typedef sendRawIxTransaction
   * @param args - command arguments
   * @param {string} args.rawIxTransaction - transaction to send
   * @return {Promise<string>} - transaction id
   */
  async function sendRawIxTransaction(args) {
    validator.validate(args);
    const { rawTransaction } = args;
    return coreAPI.sendRawIxTransaction(rawTransaction);
  }

  return sendRawIxTransaction;
};

module.exports = sendRawIxTransactionFactory;
