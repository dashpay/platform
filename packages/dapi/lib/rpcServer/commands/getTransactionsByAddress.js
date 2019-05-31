const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/addresses');
const ArgumentsValidationError = require('../../errors/ArgumentsValidationError');

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
   * @param {string|string[]} args.address
   * @param {number} args.from
   * @param {number} args.to
   * @param {number} args.fromHeight
   * @param {number} args.toHeight
   * @return {Promise<Array<Object>>}
   */
  async function getTransactionsByAddress(args) {
    validator.validate(args);
    const {
      address, from, to, fromHeight, toHeight,
    } = args;
    if (from !== undefined && to !== undefined && to - from > 50) {
      throw new ArgumentsValidationError(`"from" (${from}) and "to" (${to}) range should be less than or equal to 50`);
    }
    return coreAPI.getTransactionsByAddress(address, from, to, fromHeight, toHeight);
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
 *          description: Successful response. Promise (object) containing all transaction objects for the requested address.
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
 *                      oneOf:
 *                        - type: string
 *                          description: Dash address
 *                        - type: array
 *                          items:
 *                            type: string
 *                            description: Array of Dash addresses
 *                    from:
 *                      type: integer
 *                      description: Start of range in the ordered list of latest UTXO
 *                    to:
 *                      type: integer
 *                      description: End of range in the ordered list of latest UTXO
 *                    fromHeight:
 *                      type: integer
 *                      description: Lowest block height to include
 *                    toHeight:
 *                      type: integer
 *                      description: Block height to end on
 */
/* eslint-enable max-len */

module.exports = getTransactionsByAddressFactory;
