const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getQuorum');

const validator = new Validator(argsSchema);
/**
 * @param coreApi
 * @return {getQuorum}
 */
const getQuorumFactory = (coreApi) => {
  /**
   * Layer 1 endpoint
   * Returns user quorum (llmq)
   * @typedef getQuorum
   * @param args - command arguments
   * @param {string} args.regTxId
   * @return {Promise<object>}
   */
  async function getQuorum(args) {
    validator.validate(args);
    const { regTxId } = args;
    return coreApi.getQuorum(regTxId);
  }

  return getQuorum;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getQuorum:
 *   post:
 *      operationId: getQuorum
 *      deprecated: false
 *      summary: getQuorum
 *      description: Returns a user quorum (LLMQ)
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing a user quorum.
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
 *                  default: getQuorum
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
 *                    - regTxId
 *                  properties:
 *                    regTxId:
 *                      type: string
 *                      default: '0000000000000000000000000000000000000000000000000000000000000000'
 *                      description: The TXID of the user's registration subscription transaction
 */
/* eslint-enable max-len */

module.exports = getQuorumFactory;
