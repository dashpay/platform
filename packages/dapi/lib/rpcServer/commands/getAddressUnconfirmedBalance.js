const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getAddressUnconfirmedBalance}
 */
const getAddressUnconfirmedBalanceFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns total unconfirmed balance for the address
   * @typedef getAddressUnconfirmedBalance
   * @param args - command arguments
   * @param {string} args.address
   * @return {Promise<number>}
   */
  async function getAddressUnconfirmedBalance(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getAddressUnconfirmedBalance(address);
  }

  return getAddressUnconfirmedBalance;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getAddressUnconfirmedBalance:
 *   post:
 *      operationId: getAddressUnconfirmedBalance
 *      deprecated: false
 *      summary: getAddressUnconfirmedBalance
 *      description: Get the total unconfirmed balance for the address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (number) indicating the number of unconfirmed duffs for the address.
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
 *                  default: getAddressUnconfirmedBalance
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

module.exports = getAddressUnconfirmedBalanceFactory;
