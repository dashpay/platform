const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getUserDapSpace');

const validator = new Validator(argsSchema);
/**
 * @param dashDrive
 * @return {getUserDapSpace}
 */
const getUserDapSpaceFactory = (dashDrive) => {
  /**
   * Layer 2 endpoint
   * Returns user dap space
   * @typedef getUserDapSpace
   * @param args - command arguments
   * @param {string} args.dapId
   * @param {string} args.userId
   * @return {Promise<object>}
   */
  async function getUserDapSpace(args) {
    validator.validate(args);
    const { dapId, userId } = args;
    return dashDrive.getDapSpace(dapId, userId);
  }

  return getUserDapSpace;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getUserDapSpace:
 *   post:
 *      operationId: getUserDapSpace
 *      deprecated: false
 *      summary: getUserDapSpace
 *      description: Returns a user DAP Space
 *      tags:
 *        - L2
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing details of a user's DAP space.
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
 *                  default: getUserDapSpace
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
 *                    - dapid
 *                    - userId
 *                  properties:
 *                    dapid:
 *                      type: string
 *                      default: ''
 *                      description: The ID of a DAP the user is registered with
 *                    userId:
 *                      type: string
 *                      default: ''
 *                      description: ID of the user
 */
/* eslint-enable max-len */

module.exports = getUserDapSpaceFactory;
