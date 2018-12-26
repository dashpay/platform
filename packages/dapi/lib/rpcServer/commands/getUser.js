const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/getUser');

const validator = new Validator(argsSchema);

/**
 * @param coreAPI
 * @return {getUser}
 */
const getUserFactory = (coreAPI) => {
  /**
   * Layer 2 endpoint
   * Returns blockchain user
   * @typedef getUser
   * @param args
   * @param {string} args.usernameOrRegTxId
   * @return {Promise<object>}
   */
  async function getUser(args) {
    validator.validate(args);
    const { username, userId } = args;
    return coreAPI.getUser(username || userId);
  }

  return getUser;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getUser:
 *   post:
 *      operationId: getUser
 *      deprecated: false
 *      summary: getUser
 *      description: Returns a blockchain user
 *      tags:
 *        - L2
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing user info.
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
 *                  default: getUser
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
 *                    - username
 *                  properties:
 *                    username:
 *                      type: string
 *                      default: DashUser001
 *                      description: Either a username or a user's registration tx id
 */
/* eslint-enable max-len */

module.exports = getUserFactory;
