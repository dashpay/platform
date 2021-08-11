const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlockHash.json');

const validator = new Validator(argsSchema);

/**
 * @param {Object} coreAPI
 * @return {getBlockHash}
 */
const getBlockHashFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns block hash for the given height
   * @typedef getBlockHash
   * @param args
   * @param {number} args.height - block height
   * @return {Promise<string>} - block hash
   */
  async function getBlockHash(args) {
    validator.validate(args);
    const { height } = args;
    return coreAPI.getBlockHash(height);
  }

  return getBlockHash;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getBlockHash:
 *   post:
 *      operationId: getBlockHash
 *      deprecated: false
 *      summary: getBlockHash
 *      description: Returns the block hash for the given height
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (string) containing the requested block hash.
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
 *                  default: getBlockHash
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
 *                    - height
 *                  properties:
 *                    height:
 *                      type: integer
 *                      default: 1
 *                      description: Block height
 *                      minimum: 0
 */
/* eslint-enable max-len */

module.exports = getBlockHashFactory;
