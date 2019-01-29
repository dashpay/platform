const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/sendRawTransaction');

const validator = new Validator(argsSchema);
/**
 * Sends raw transaction to the network
 * @param coreAPI
 * @return {sendRawTransaction}
 */
const sendRawTransactionFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * sends raw transaction
   * @typedef sendRawTransaction
   * @param args - command arguments
   * @param {string} args.rawTransaction - transaction to send
   * @return {Promise<string>} - transaction id
   */
  async function sendRawTransaction(args) {
    validator.validate(args);
    const {
      rawTransaction, allowHighFees, instantSend, bypassLimits,
    } = args;
    return coreAPI.sendRawTransaction(rawTransaction, allowHighFees, instantSend, bypassLimits);
  }

  return sendRawTransaction;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /sendRawTransaction:
 *   post:
 *      operationId: sendRawTransaction
 *      deprecated: false
 *      summary: sendRawTransaction
 *      description: Sends raw transaction and returns the transaction ID
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (string array) containing the transaction ID.
 *      requestBody:
 *        content:
 *          application/json:
 *            schema:
 *              type: object
 *              required:
 *                - method
 *                - id
 *                - jsonrpc
 *                - params
 *              properties:
 *                method:
 *                  type: string
 *                  default: sendRawTransaction
 *                  description: Method name
 *                id:
 *                  type: integer
 *                  default: 1
 *                  format: int32
 *                  description: Request ID
 *                jsonrpc:
 *                  type: string
 *                  default: '2.0'
 *                  description: JSON-RPC Version (2.0)
 *                params:
 *                  title: Parameters
 *                  type: object
 *                  required:
 *                    - rawTransaction
 *                  properties:
 *                    rawTransaction:
 *                      type: string
 *                      default: ''
 *                      description: Raw transaction
 */
/* eslint-enable max-len */

module.exports = sendRawTransactionFactory;
