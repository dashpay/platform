const Validator = require('../../utils/Validator');
// TODO: Add name validation
const argsSchema = require('../schemas/searchUsers');

const validator = new Validator(argsSchema);

/**
 * @param userIndex
 * @return {function({pattern: string, limit: number, offset: number}): {totalCount: *, results: *}}
 */
const searchSearchUsersFactory = (userIndex) => {
  /**
   * Layer 2 endpoint
   * @param args
   * @param {string} args.pattern
   * @param {number} args.limit
   * @param {number} args.offset
   * @return {Promise<{totalCount: number, results: Array<string>}>}
   */
  async function searchUsers(args) {
    validator.validate(args);
    const { pattern, limit, offset } = args;
    const usernames = await userIndex.searchUsernames(pattern);
    return {
      totalCount: usernames.length,
      results: usernames.slice(offset, offset + limit),
    };
  }

  return searchUsers;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /searchUsers:
 *   post:
 *      operationId: searchUsers
 *      deprecated: false
 *      summary: searchUsers
 *      description: Adds an element to an existing bloom filter
 *      tags:
 *        - L2
 *      responses:
 *        200:
 *          description: Successful response. Promise (object with array of strings) containing the number of results returned and an array of matching users.
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
 *                  default: searchUsers
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
 *                    - limit
 *                    - offset
 *                  properties:
 *                    pattern:
 *                      type: string
 *                      default: 'DashUser001'
 *                      description: Search pattern
 *                    limit:
 *                      type: integer
 *                      default: 10
 *                      description: Maximum number of results to return
 *                      minimum: 1
 *                      maximum: 25
 *                    offset:
 *                      type: integer
 *                      default: 0
 *                      description: Starting location in result set
 *                      minimum: 0
 */
/* eslint-enable max-len */

module.exports = searchSearchUsersFactory;
