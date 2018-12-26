const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getStatus');

const validator = new Validator(argsSchema);
/**
 * @param coreAPI
 * @return {getStatus}
 */
const getStatusFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns calculated balance for the address
   * @typedef getStatus
   * @param args - command arguments
   * @param {string} args.query
   * @return {Promise<*>}
   */
  async function getStatus(args) {
    validator.validate(args);
    const { query } = args;
    return coreAPI.getStatus(query);
  }

  return getStatus;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getStatus:
 *   post:
 *      operationId: getStatus
 *      deprecated: false
 *      summary: getStatus
 *      description: Returns status based on query parameter
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Status details for the provided query.
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
 *                  default: getStatus
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
 *                    - query
 *                  properties:
 *                    query:
 *                      type: string
 *                      default: getInfo
 *                      description: Type of status to get (getInfo, getDifficulty, getBestBlockHash, or getLastBlockHash)
 */
/* eslint-enable max-len */

module.exports = getStatusFactory;
