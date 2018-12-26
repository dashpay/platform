const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/address');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getAddressTotalReceived}
 */
const getAddressTotalReceivedFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns total amount of duffs received by address
   * @typedef getAddressTotalReceived
   * @param args - command arguments
   * @param {string} args.address
   * @return {Promise<number>}
   */
  async function getAddressTotalReceived(args) {
    validator.validate(args);
    const { address } = args;
    return coreAPI.getAddressTotalReceived(address);
  }

  return getAddressTotalReceived;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getAddressTotalReceived:
 *   post:
 *      operationId: getAddressTotalReceived
 *      deprecated: false
 *      summary: getAddressTotalReceived
 *      description: Get the total amount of duffs received by an address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (number) indicating the number of duffs received by the address.
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
 *                  default: getAddressTotalReceived
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

module.exports = getAddressTotalReceivedFactory;
