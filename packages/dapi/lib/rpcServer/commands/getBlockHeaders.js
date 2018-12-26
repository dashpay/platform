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
   * @param {number} args.offset - block height starting point
   * @param {number} args.limit - number of block headers to return
   * @return {Promise<Array<Object>>}
   */
  async function getBlockHeaders(args) {
    validator.validate(args);
    const { offset, limit } = args;
    return coreAPI.getBlockHeaders(typeof offset === 'number' ? await coreAPI.getBlockHash(offset) : offset, limit);
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
 *                      type: integer
 *                      default: 1
 *                      description: Lowest block height to include
 *                      minimum: 0
 *                    limit:
 *                      type: integer
 *                      default: 1
 *                      description: The number of headers to return (0 < limit <=25)
 *                      minimum: 1
 *                      maximum: 25
 */
/* eslint-enable max-len */

module.exports = getBlockHeadersFactory;
