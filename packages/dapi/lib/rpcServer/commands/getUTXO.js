const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/addresses');

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
 *                        type: string
 *                        type: array
 *                        items:
 *                          type: string
 *                        required: true
 *                        description: Dash address or array of addresses
 *                    from:
 *                      type: integer
 *                      default: undefined
 *                      description: start of range in the ordered list of latest UTXO
 *                    to:
 *                      type: integer
 *                      default: undefined
 *                      description: end of range in the ordered list of latest UTXO
 *                    fromHeight:
 *                      type: integer
 *                      default: undefined
 *                      description: which height to start from
 *                    toHeight:
 *                      type: integer
 *                      default: undefined
 *                      description: on which height to end
 */
/* eslint-enable max-len */

module.exports = getUTXOFactory;
