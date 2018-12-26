const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/fetchDapContract');

const validator = new Validator(argsSchema);
/**
 * @param {AbstractDashDriveAdapter} dashDriveAPI
 * @return {fetchDapContract}
 */
const fetchDapContractFactory = (dashDriveAPI) => {
  /**
   * Layer 2 endpoint
   * Returns user dap space
   * @typedef fetchDapContract
   * @param args - command arguments
   * @param {string} args.dapId
   * @return {Promise<object>}
   */
  async function fetchDapContract(args) {
    validator.validate(args);
    const { dapId } = args;
    return dashDriveAPI.fetchDapContract(dapId);
  }

  return fetchDapContract;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /fetchDapContract:
 *   post:
 *      operationId: fetchDapContract
 *      deprecated: false
 *      summary: fetchDapContract
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
 *                  default: fetchDapContract
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
 *                    - dapId
 *                  properties:
 *                    dapId:
 *                      type: string
 *                      default: ''
 *                      description: A user's DAP ID
 */
/* eslint-enable max-len */

module.exports = fetchDapContractFactory;
