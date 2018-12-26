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
   * Layer 1 endpoint
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

/* eslint-disable max-len */
/**
 * @swagger
 * /getBalance:
 *   post:
 *      operationId: getBalance
 *      deprecated: false
 *      summary: getBalance
 *      description: Get the calculated balance for the address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (number) indicating the balance of the address.
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
 *                  default: getBalance
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

module.exports = getBalanceFactory;
