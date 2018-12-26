const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getTransactionById');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getTransactionById}
 */
const getTransactionByIdFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
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

/* eslint-disable max-len */
/**
 * @swagger
 * /getTransactionById:
 *   post:
 *      operationId: getTransactionById
 *      deprecated: false
 *      summary: getTransactionById
 *      description: Returns transaction for the given hash
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing transaction info.
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
 *                  default: getTransactionById
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
 *                    - txid
 *                  properties:
 *                    txid:
 *                      type: string
 *                      default: '0000000000000000000000000000000000000000000000000000000000000000'
 *                      description: The TXID of the transaction being requested
 */
/* eslint-enable max-len */

module.exports = getTransactionByIdFactory;
