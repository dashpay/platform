const {
  v0: {
    GetIdentityNonceResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getIdentityNonceHandler}
 */
function getIdentityNonceHandlerFactory(driveClient) {
  /**
   * @typedef getIdentityNonceHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentityNonceResponse>}
   */
  async function getIdentityNonceHandler(call) {
    const { request } = call;

    const identityNonceBuffer = await driveClient
      .fetchIdentityNonce(request);

    return GetIdentityNonceResponse
      .deserializeBinary(identityNonceBuffer);
  }

  return getIdentityNonceHandler;
}

module.exports = getIdentityNonceHandlerFactory;
