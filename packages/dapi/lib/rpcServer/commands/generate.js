const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/generate');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {generate}
 */
const generateFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * WORKS ONLY IN REGTEST MODE.
   * Generates blocks on demand for regression tests.
   * @typedef generate
   * @param args - command arguments
   * @param {number} args.amount - amount of blocks to generate
   * @return {Promise<string[]>} - generated block hashes
   */
  async function generate(args) {
    validator.validate(args);
    const { amount } = args;
    return coreAPI.generate(amount);
  }

  return generate;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /generate:
 *   post:
 *      operationId: generate
 *      deprecated: false
 *      summary: generate
 *      description: Generates blocks on demand
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (string array) containing strings of block hashes.
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
 *                  default: generate
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
 *                    - amount
 *                  properties:
 *                    amount:
 *                      type: integer
 *                      default: 1
 *                      description: Number of blocks to generate
 */
/* eslint-enable max-len */

module.exports = generateFactory;
