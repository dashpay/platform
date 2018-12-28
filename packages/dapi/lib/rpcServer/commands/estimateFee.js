const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/estimateFee');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {estimateFee}
 */
const estimateFeeFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * estimates fee
   * @typedef estimateFee
   * @param {object} args
   * @param {number} args.blocks - target
   * @return {Promise<number>} - fee in duffs per kilobyte
   */
  async function estimateFee(args) {
    validator.validate(args);
    const { blocks } = args;
    return coreAPI.estimateFee(blocks);
  }

  return estimateFee;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /estimateFee:
 *   post:
 *      operationId: estimateFee
 *      deprecated: false
 *      summary: estimateFee
 *      description: Estimates the transaction fee necessary for confirmation to occur within a certain number of blocks
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (number) containing fee in duffs per kilobyte.
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
 *                  default: estimateFee
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
 *                    - blocks
 *                  properties:
 *                    blocks:
 *                      type: integer
 *                      default: 1
 *                      description: Number of blocks for fee estimate
 */
/* eslint-enable max-len */

module.exports = estimateFeeFactory;
