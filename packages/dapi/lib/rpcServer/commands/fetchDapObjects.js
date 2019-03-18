const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/fetchDapObjects');

const validator = new Validator(argsSchema);
/**
 * @param {AbstractDashDriveAdapter} dashDriveAPI
 * @return {fetchDapObjects}
 */
const fetchDapObjectsFactory = (dashDriveAPI) => {
  /**
   * Layer 2 endpoint
   * Fetches user objects for a given condition
   * @typedef fetchDapObjects
   * @param args - command arguments
   * @param {string} args.contractId
   * @param {string} args.type
   * @param args.options
   * @param {Object} args.options.where - Mongo-like query
   * @param {Object} args.options.orderBy - Mongo-like sort field
   * @param {number} args.options.limit - how many objects to fetch
   * @param {number} args.options.startAt - number of objects to skip
   * @param {number} args.options.startAfter - exclusive skip
   * @return {Promise<object>}
   */
  async function fetchDapObjects(args) {
    validator.validate(args);
    const { contractId, type, options } = args;
    return dashDriveAPI.fetchDapObjects(contractId, type, options);
  }

  return fetchDapObjects;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /fetchDapObjects:
 *   post:
 *      operationId: fetchDapObjects
 *      deprecated: false
 *      summary: fetchDapObjects
 *      description: Fetches user objects for a given condition
 *      tags:
 *        - L2
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) with user objects.
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
 *                  default: fetchDapObjects
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
 *                    - contractId
 *                    - type
 *                  properties:
 *                    contractId:
 *                      type: string
 *                      default: ''
 *                      description: A user's contract ID
 *                    type:
 *                      type: string
 *                      default: ''
 *                      description: DAP object type to fetch
 *                    options:
 *                      title: Options
 *                      type: object
 *                      properties:
 *                        where:
 *                          type: string
 *                          default: ''
 *                          description: Mongo-like query
 *                        orderBy:
 *                          type: string
 *                          default: ''
 *                          description: Mongo-like sort field
 *                        limit:
 *                          type: integer
 *                          default: ''
 *                          description: How many objects to fetch
 *                        startAt:
 *                          type: integer
 *                          default: ''
 *                          description: Number of objects to skip
 *                        startAfter:
 *                          type: integer
 *                          default: ''
 *                          description: Exlusive skip
 */
/* eslint-enable max-len */

module.exports = fetchDapObjectsFactory;
