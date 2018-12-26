const BloomFilter = require('bloom-filter');
const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getSpvData');

const validator = new Validator(argsSchema);
/**
 * @param spvService
 * @return {getSpvData}
 */
const getSpvDataFactory = (spvService) => {
  /**
   * Layer 1 endpoint
   * Returns block headers
   * @typedef getSpvData
   * @param args - command arguments
   * @param {string} args.filter
   * @return {object}
   */
  async function getSpvData(args) {
    validator.validate(args);
    const filter = new BloomFilter(args.filter);
    return spvService.getSpvData(filter);
  }

  return getSpvData;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getSpvData:
 *   post:
 *      operationId: getSpvData
 *      deprecated: false
 *      summary: getSpvData
 *      description: Returns block headers
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
 *                  default: getSpvData
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

module.exports = getSpvDataFactory;
