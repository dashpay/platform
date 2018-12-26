/**
 * @param coreAPI
 * @return {getBestBlockHeight}
 */
const getBestBlockHeightFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns best block height
   * @typedef getBestBlockHeight
   * @return {Promise<number>} - best seen block height
   */
  async function getBestBlockHeight() {
    return coreAPI.getBestBlockHeight();
  }

  return getBestBlockHeight;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getBestBlockHeight:
 *   post:
 *      operationId: getbestblockheight
 *      deprecated: false
 *      summary: getBestBlockHeight
 *      description: Returns the best block height
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise indicating the best seen block height.
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
 *                  default: getBestBlockHeight
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

module.exports = getBestBlockHeightFactory;
