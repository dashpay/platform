const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getAddressTotalSent}
 */
const getAddressTotalSentFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns total amount of duffs sent by the address
   * @typedef getAddressTotalSent
   * @param args - command arguments
   * @param {string} args.address
   * @return {Promise<number>}
   */
  async function getAddressTotalSent(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getAddressTotalSent(address);
  }

  return getAddressTotalSent;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getAddressTotalSent:
 *   post:
 *      operationId: getAddressTotalSent
 *      deprecated: false
 *      summary: getAddressTotalSent
 *      description: Get the total amount of duffs sent by an address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (number) indicating the number of duffs sent by the address.
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
 *                  default: getAddressTotalSent
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

module.exports = getAddressTotalSentFactory;
