const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlocks');

const validator = new Validator(argsSchema);
/**
 * Returns getBlocks function
 * @param coreAPI
 * @return {getBlocks}
 */
const getBlocksFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns info for blocks
   * @typedef getBlocks
   * @param args - command arguments
   * @param {string} args.blockDate
   * @param {number} args.limit - number of blocks to return
   * @return {Promise<object[]>}
   */
  async function getBlocks(args) {
    validator.validate(args);
    const { limit, blockDate } = args;
    return coreAPI.getBlocks(blockDate, limit);
  }

  return getBlocks;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getBlocks:
 *   post:
 *      operationId: getBlocks
 *      deprecated: false
 *      summary: getBlocks
 *      description: Returns info for blocks meeting the provided criteria
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object array) containing an array of blocks.
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
 *                  default: getBlocks
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
 *                    - blockDate
 *                    - limit
 *                  properties:
 *                    blockDate:
 *                      type: string
 *                      default: 2018-06-01
 *                      description: Starting date for blocks to get
 *                    limit:
 *                      type: integer
 *                      default: 1
 *                      description: Number of blocks to return
 *                      minimum: 1
 */
/* eslint-enable max-len */

module.exports = getBlocksFactory;
