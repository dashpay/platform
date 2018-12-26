const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/clearBloomFilter');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {clearBloomFilter}
 */
const clearBloomFilterFactory = (spvService) => {
  /**
   * Layer 1 endpoint
   * clears bloom filter
   * @typedef clearBloomFilter
   * @param args - command arguments
   * @param {string} args.filter
   * @return {Promise<bool>}
   */
  async function clearBloomFilter(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.clearBloomFilter(filter);
  }

  return clearBloomFilter;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /clearBloomFilter:
 *   post:
 *      operationId: clearBloomFilter
 *      deprecated: false
 *      summary: clearBloomFilter
 *      description: Clear the bloom filter
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (boolean) indicating if the filter was successfully cleared.
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
 *                  default: clearBloomFilter
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
 *                    originalFilter:
 *                      type: string
 *                      default: ''
 *                      description: Original filter
 */
/* eslint-enable max-len */

module.exports = clearBloomFilterFactory;
