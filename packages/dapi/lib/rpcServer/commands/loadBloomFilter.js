const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/loadBloomFilter');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {loadBloomFilter}
 */
const loadBloomFilterFactory = (spvService) => {
  /**
   * Layer 1 endpoint
   * Returns boolean
   * @typedef loadBloomFilter
   * @param args - command arguments
   * @param {string} args.filter
   * @return {Promise<bool>}
   */
  async function loadBloomFilter(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.loadBloomFilter(filter);
  }

  return loadBloomFilter;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /loadBloomFilter:
 *   post:
 *      operationId: loadBloomFilter
 *      deprecated: false
 *      summary: loadBloomFilter
 *      description: Load a bloom filter
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (boolean) indicating if the filter was successfully loaded.
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
 *                  default: loadBloomFilter
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
 *                      description: Filter to load
 */
/* eslint-enable max-len */

module.exports = loadBloomFilterFactory;
