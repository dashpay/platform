const Validator = require('../../utils/Validator');
// TODO: Add name validation
const argsSchema = require('../schemas/searchDapContracts');

const validator = new Validator(argsSchema);

/**
 * @param dashDrive
 * @return {function({pattern: string}): Promise<Array<string>>}
 */
const searchDapContractsFactory = (dashDrive) => {
  /**
   * Layer 2 endpoint
   * Returns array of dap ids
   * @param args
   * @param {string} args.pattern
   * @return {Promise<Array<string>>}
   */
  async function searchDapContracts(args) {
    validator.validate(args);
    const { pattern } = args;
    return dashDrive.searchDapContracts(pattern);
  }

  return searchDapContracts;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /searchDapContracts:
 *   post:
 *      operationId: searchDapContracts
 *      deprecated: false
 *      summary: searchDapContracts
 *      description: Returns an array of DAP IDs
 *      tags:
 *        - L2
 *      responses:
 *        200:
 *          description: Successful response. Promise (string array) containing an array of DAP IDs.
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
 *                  default: searchDapContracts
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
 *                    - pattern
 *                  properties:
 *                    pattern:
 *                      type: string
 *                      default: ''
 *                      description: Search pattern
 */
/* eslint-enable max-len */

module.exports = searchDapContractsFactory;
