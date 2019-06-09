const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlockHeaders');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {getBlockHeaders}
 */
const getBlockHeadersFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns block headers
   * @typedef getBlockHeaders
   * @param args - command arguments
   * @param {number|string}  args.offset - block height/hash starting point
   * @param {number} [args.limit=1] - number of block headers to return
   * @param {boolean} [args.verbose=false] - set true to get headers as deserialized json
   * @return {Promise<Array<Object>>}
   */
  async function getBlockHeaders(args) {
    validator.validate(args);
    const { offset, limit, verbose } = args;
    const hash = typeof offset === 'number' ? await coreAPI.getBlockHash(offset) : offset;
    return coreAPI.getBlockHeaders(hash, limit, verbose);
  }

  return getBlockHeaders;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getBlockHeaders:
 *   post:
 *      operationId: getBlockHeaders
 *      deprecated: false
 *      summary: getBlockHeaders
 *      description: Returns the requested number of block headers starting at the requested height
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object array) containing an array of block headers.
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
 *                  default: getBlockHeaders
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
 *                    - offset
 *                    - limit
 *                  properties:
 *                    offset:
 *                      oneOf:
 *                        - type: integer
 *                          description: Lowest block height to include
 *                        - type: string
 *                          description: Lowest block hash to include
 *                    limit:
 *                      type: integer
 *                      default: 1
 *                      description: The number of headers to return (0 < limit <=2000)
 *                      minimum: 1
 *                      maximum: 2000
 *                    verbose:
 *                      type: boolean
 *                      default: false
 *                      description: set true to get headers as deserialized json
 */
/* eslint-enable max-len */

module.exports = getBlockHeadersFactory;
