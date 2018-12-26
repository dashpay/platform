const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getTransactionsByAddress}
 */
const getTransactionsByAddressFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
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

/* eslint-disable max-len */
/**
 * @swagger
 * /getTransactionsByAddress:
 *   post:
 *      operationId: getTransactionsByAddress
 *      deprecated: false
 *      summary: getTransactionsByAddress
 *      description: Returns all transaction related to the given address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object array) containing all transaction objects for the requested address.
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
 *                  default: getTransactionsByAddress
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
 *                    - address
 *                  properties:
 *                    address:
 *                      type: string
 *                      default: yLp6ZJueuigiF4s9E1Pv8tEunDPEsjyQfd
 *                      description: Dash address
 */
/* eslint-enable max-len */

module.exports = getTransactionsByAddressFactory;
