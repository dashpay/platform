const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getRawBlock');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getRawBlock}
 */
const getRawBlockFactory = (coreAPI) => {
  // Todo: document summary format
  /**
   * Layer 1 endpoint
   * Returns raw block for the given block hash
   * @typedef getRawBlock
   * @param args
   * @param {string} args.blockHash
   * @return {Promise<object>}
   */
  async function getRawBlock(args) {
    validator.validate(args);
    const { blockHash } = args;
    return coreAPI.getRawBlock(blockHash);
  }

  return getRawBlock;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getRawBlock:
 *   post:
 *      operationId: getRawBlock
 *      deprecated: false
 *      summary: getRawBlock
 *      description: Returns the raw block for the provided block hash
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing the raw block details.
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
 *                  default: getRawBlock
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
 *                    - blockHash
 *                  properties:
 *                    blockHash:
 *                      type: string
 *                      default: '0000000000000000000000000000000000000000000000000000000000000000'
 *                      description: Hash of the block to request
 */
/* eslint-enable max-len */

module.exports = getRawBlockFactory;
