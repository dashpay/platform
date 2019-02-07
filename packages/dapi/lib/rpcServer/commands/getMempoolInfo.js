/**
 * @param {Object} coreAPI
 * @return {getMempoolInfo}
 */
const getMempoolInfoFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns info about the mempool usage
   * @typedef getMempoolInfo
   * @return {Promise<Object>} - mempool info object
   */
  async function getMempoolInfo() {
    return coreAPI.getMempoolInfo();
  }

  return getMempoolInfo;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getMempoolInfo:
 *   post:
 *      operationId: getMempoolInfo
 *      deprecated: false
 *      summary: getMempoolInfo
 *      description: Returns mempool info object
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (Object) containing info about mempool usage.
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
 *                  default: getMempoolInfo
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

module.exports = getMempoolInfoFactory;
