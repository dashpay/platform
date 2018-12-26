/**
 * @param coreAPI
 * @return {getMNList}
 */
const getMNListFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns masternode list
   * @typedef getMNList
   * @return {Promise<object[]>}
   */
  async function getMNList() {
    const insightMNList = await coreAPI.getMasternodesList();
    return insightMNList.map(masternode => Object.assign(masternode, { ip: masternode.ip.split(':')[0] }));
  }

  return getMNList;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getMNList:
 *   post:
 *      operationId: getMNList
 *      deprecated: false
 *      summary: getMNList
 *      description: "Returns masternode list"
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object array) containing masternode list.
 *      requestBody:
 *        content:
 *          application/json:
 *            schema:
 *              type: object
 *              required:
 *                - method
 *                - id
 *                - jsonrpc
 *              properties:
 *                method:
 *                  type: string
 *                  default: getMNList
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
 */
/* eslint-enable max-len */

module.exports = getMNListFactory;
