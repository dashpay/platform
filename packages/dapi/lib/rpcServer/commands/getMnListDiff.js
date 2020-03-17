const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getMnListDiff');

const validator = new Validator(argsSchema);
/**
 * Returns getMnListDiff function
 * @param coreAPI
 * @return {getMnListDiff}
 */
const getMnListDiffFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns calculated balance for the address
   * @typedef getMnListDiff
   * @param args - command arguments
   * @param baseBlockHash {string}
   * @param blockHash {string}
   * @return {Promise<string>}
   */
  async function getMnListDiff(args) {
    validator.validate(args);
    const { baseBlockHash, blockHash } = args;

    return coreAPI.getMnListDiff(baseBlockHash, blockHash);
  }

  return getMnListDiff;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getMnListDiff:
 *   post:
 *      operationId: getMnListDiff
 *      deprecated: false
 *      summary: getMnListDiff
 *      description: "Returns masternode list diff for the provided block hashes"
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object array) containing a diff of the masternode list based on the provided block hashes.
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
 *                  default: getMnListDiff
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
 *                    - baseBlockHash
 *                    - blockHash
 *                  properties:
 *                    baseBlockHash:
 *                      type: string
 *                      default: '0000000000000000000000000000000000000000000000000000000000000000'
 *                      description: Block hash
 *                    blockHash:
 *                      type: string
 *                      default: '0000000000000000000000000000000000000000000000000000000000000000'
 *                      description: Block hash
 */
/* eslint-enable max-len */

module.exports = getMnListDiffFactory;
