const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/addToBloomFilter');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {addToBloomFilter}
 */
const addToBloomFilterFactory = (spvService) => {
  /**
   * Layer 1 endpoint
   * adds an element to an existing bloom filter
   * @typedef addToBloomFilter
   * @param args - command arguments
   * @param {string} args.originalFilter
   * @param {string} args.element
   * @return {Promise<bool>}
   */
  async function addToBloomFilter(args) {
    validator.validate(args);
    const originalFilter = new BloomFilter(args.originalFilter);
    const { element } = args;
    return spvService.addToBloomFilter(originalFilter, element);
  }

  return addToBloomFilter;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /addToBloomFilter:
 *   post:
 *      operationId: addToBloomFilter
 *      deprecated: false
 *      summary: addToBloomFilter
 *      description: Adds an element to an existing bloom filter
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (boolean) indicating if the element was successfully added.
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
 *                  default: addToBloomFilter
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
 *                    - originalFilter
 *                    - element
 *                  properties:
 *                    originalFilter:
 *                      type: string
 *                      default: ''
 *                      description: Original filter
 *                    element:
 *                      type: string
 *                      default: ''
 *                      description: Element to add to filter
 */
/* eslint-enable max-len */

module.exports = addToBloomFilterFactory;
