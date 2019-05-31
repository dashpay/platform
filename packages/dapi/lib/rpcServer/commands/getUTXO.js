const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/addresses');
const ArgumentsValidationError = require('../../errors/ArgumentsValidationError');

const validator = new Validator(argsSchema);


/**
 * @param coreAPI
 * @return {getUTXO}
 */
const getUTXOFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns unspent outputs for the given address
   * @typedef getUTXO
   * @param args
   * @param {string|string[]} args.address
   * @param {number} args.from
   * @param {number} args.to
   * @param {number} args.fromHeight
   * @param {number} args.toHeight
   * @return {Promise<Array<Object>>}
   */
  async function getUTXO(args) {
    validator.validate(args);
    const {
      address, from, to, fromHeight, toHeight,
    } = args;
    if (from !== undefined && to !== undefined && to - from > 1000) {
      throw new ArgumentsValidationError(`"from" (${from}) and "to" (${to}) range should be less than or equal to 1000`);
    }
    return coreAPI.getUTXO(address, from, to, fromHeight, toHeight);
  }

  return getUTXO;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getUTXO:
 *   post:
 *      operationId: getUTXO
 *      deprecated: false
 *      summary: getUTXO
 *      description: Returns unspent outputs for the given address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing unspent transaction objects.
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
 *                  default: getUTXO
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

module.exports = getUTXOFactory;
