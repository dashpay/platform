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
   * Layer 1 endpoint
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

/* eslint-disable max-len */
/**
 * @swagger
 * /sendRawIxTransaction:
 *   post:
 *      operationId: sendRawIxTransaction
 *      deprecated: false
 *      summary: sendRawIxTransaction
 *      description: Sends raw InstantSend transaction and returns the transaction ID
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
 *                  default: sendRawIxTransaction
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
 *                    - rawIxTransaction
 *                  properties:
 *                    rawIxTransaction:
 *                      type: string
 *                      default: ''
 *                      description: Raw InstantSend transaction
 */
/* eslint-enable max-len */

module.exports = sendRawIxTransactionFactory;
