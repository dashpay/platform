const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getSpvData');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {findSpvData}
 */
const findDataForBlockFactory = (spvService) => {
  /**
   * Layer 1 endpoint
   * finds relevant data for addresses encoded in a bloom filter in a specific block
   * @typedef findDataForBlock
   * @param args - command arguments
   * @param {string} args.filter
   * @return {object}
   */
  async function findDataForBlock(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.findDataForBlock(filter, args.blockHash);
  }

  return findDataForBlock;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /findDataForBlock:
 *   post:
 *      operationId: findDataForBlock
 *      deprecated: false
 *      summary: findDataForBlock
 *      description: Finds relevant data in a specific block for addresses encoded in a bloom filter
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. An object containing block headers.
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
 *                  default: findDataForBlock
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
 *                    - filter
 *                  properties:
 *                    filter:
 *                      type: string
 *                      default: ''
 *                      description: A bloom filter
 */
/* eslint-enable max-len */

module.exports = findDataForBlockFactory;
