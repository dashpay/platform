/**
 * @param {Object} coreAPI
 * @param {ZmqClient} coreZmqClient
 * @return {getBestBlockHash}
 */
const getBestBlockHashFactory = (coreAPI, coreZmqClient) => {
  let hash = null;

  // Reset height on a new block, so it will be obtain again on a user request
  coreZmqClient.on(
    coreZmqClient.topics.hashblock,
    () => {
      hash = null;
    },
  );

  /**
   * Layer 1 endpoint
   * Returns block hash of the chaintip
   * @typedef getBestBlockHash
   * @return {Promise<string>} - latest block hash
   */
  async function getBestBlockHash() {
    if (hash === null) {
      hash = await coreAPI.getBestBlockHash();
    }

    return hash;
  }

  return getBestBlockHash;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getBestBlockHash:
 *   post:
 *      operationId: getBestBlockHash
 *      deprecated: false
 *      summary: getBestBlockHash
 *      description: Returns block hash of the chaintip
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (string) containing the latest block hash.
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
 *                  default: getBestBlockHash
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

module.exports = getBestBlockHashFactory;
