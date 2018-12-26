const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getBlockHeader');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {getBlockHeader}
 */
const getBlockHeaderFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns block headers
   * @typedef getBlockHeader
   * @param args - command arguments
   * @param {string} args.blockHash
   * @return {Promise<Array<Object>>}
   */
  async function getBlockHeader(args) {
    validator.validate(args);
    const { blockHash } = args;
    return coreAPI.getBlockHeader(blockHash);
  }

  return getBlockHeader;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getBlockHeader:
 *   post:
 *      operationId: getBlockHeader
 *      deprecated: false
 *      summary: getBlockHeader
 *      description: Returns the block header corresponding to the requested block hash
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Returns the block header corresponding to the requested block hash.
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
 *                  default: getBlockHeader
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
 *                      description: Block hash
 */
/* eslint-enable max-len */

module.exports = getBlockHeaderFactory;
