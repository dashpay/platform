const Validator = require('../../utils/Validator');
const argsSchema = require('../schemas/addresses');

const RPCError = require('../RPCError');

const validator = new Validator(argsSchema);

/**
 * @param {InsightAPI} insightAPI
 * @return {getAddressSummary}
 */
const getAddressSummaryFactory = (insightAPI) => {
  /**
   * Layer 1 endpoint
   * get summary for address
   * @typedef getAddressSummary
   * @param args
   * @param {string} args.address
   * @return {Promise<object>}
   */
  async function getAddressSummary(args) {
    validator.validate(args);
    const { address } = args;

    let result;

    try {
      result = await insightAPI.getAddressSummary(address);
    } catch (e) {
      if (e.statusCode === 400) {
        throw new RPCError(-32602, e.message || 'Invalid params');
      }

      throw new RPCError(-32603, e.message || 'Internal error');
    }

    return result;
  }

  return getAddressSummary;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getAddressSummary:
 *   post:
 *      operationId: getAddressSummary
 *      deprecated: false
 *      summary: getAddressSummary
 *      description: Get an address summary given an address
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing summary details for the requested address.
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
 *                  default: getAddressSummary
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
 *                    - address
 *                  properties:
 *                    address:
 *                      oneOf:
 *                        - type: string
 *                          description: Dash address
 *                        - type: array
 *                          items:
 *                            type: string
 *                            description: Array of Dash addresses
 *                    noTxList:
 *                      type: boolean
 *                      description: if true then no tx list is returned
 *                    from:
 *                      type: integer
 *                      description: Start of range in the ordered list of latest UTXO
 *                    to:
 *                      type: integer
 *                      description: End of range in the ordered list of latest UTXO
 *                    fromHeight:
 *                      type: integer
 *                      description: Lowest block height to include
 *                    toHeight:
 *                      type: integer
 *                      description: Block height to end on
 */
/* eslint-enable max-len */

module.exports = getAddressSummaryFactory;
