const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/fetchContract');

const validator = new Validator(argsSchema);
/**
 * @param {AbstractDriveAdapter} driveAPI
 * @return {fetchContract}
 */
const fetchContractFactory = (driveAPI) => {
  /**
   * Layer 2 endpoint
   * Returns user dap space
   * @typedef fetchContract
   * @param args - command arguments
   * @param {string} args.contractId
   * @return {Promise<object>}
   */
  async function fetchContract(args) {
    validator.validate(args);
    const { contractId } = args;
    return driveAPI.fetchContract(contractId);
  }

  return fetchContract;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /fetchContract:
 *   post:
 *      operationId: fetchContract
 *      deprecated: false
 *      summary: fetchContract
 *      description: Fetch a user's DAP space
 *      tags:
 *        - L2
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) with the user's dap space.
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
 *                  default: fetchContract
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
 *                  properties:
 *                    contractId:
 *                      type: string
 *                      default: ''
 *                      description: A user's DAP ID
 */
/* eslint-enable max-len */

module.exports = fetchContractFactory;
