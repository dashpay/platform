/**
 * @param coreAPI
 * @return {getPeerDataSyncStatus}
 */
const getPeerDataSyncStatusFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * @typedef getPeerDataSyncStatus;
   * @return {Promise<object>}
   */
  function getPeerDataSyncStatus() {
    return coreAPI.getPeerDataSyncStatus();
  }

  return getPeerDataSyncStatus;
};

/* eslint-disable max-len */
/**
 * @swagger
 * /getPeerDataSyncStatus:
 *   post:
 *      operationId: getPeerDataSyncStatus
 *      deprecated: false
 *      summary: getPeerDataSyncStatus
 *      description: Returns peer data sync status
 *      tags:
 *        - L1
 *      responses:
 *        200:
 *          description: Successful response. Promise (object) containing peer data sync status.
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
 *                  default: getPeerDataSyncStatus
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

module.exports = getPeerDataSyncStatusFactory;
