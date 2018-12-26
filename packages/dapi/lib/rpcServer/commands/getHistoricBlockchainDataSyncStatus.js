/**
 * @param coreAPI
 * @return {getHistoricBlockchainDataSyncStatus}
 */
const getHistoricBlockchainDataSyncStatusFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns sync status of the node
   * @typedef getHistoricBlockchainDataSyncStatus
   * @return {Promise<object>}
   */
  function getHistoricBlockchainDataSyncStatus() {
    return coreAPI.getHistoricBlockchainDataSyncStatus();
  }

  return getHistoricBlockchainDataSyncStatus;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getHistoricBlockchainDataSyncStatus:
 *   post:
 *      operationId: getHistoricBlockchainDataSyncStatus
 *      deprecated: false
 *      summary: getHistoricBlockchainDataSyncStatus
 *      description: Returns historic blockchain data sync status
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing historical sync status.
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
 *                  default: getHistoricBlockchainDataSyncStatus
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

module.exports = getHistoricBlockchainDataSyncStatusFactory;
