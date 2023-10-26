const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @returns {getIdentityHandler}
 */
function getIdentityHandlerFactory(driveClient) {
  /**
   * @typedef getIdentityHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentityResponse>}
   */
  async function getIdentityHandler(call) {
    const { request } = call;

    if (!request.getV0().getId()) {
      throw new InvalidArgumentGrpcError('id is not specified');
    }

    const identityResponseBuffer = await driveClient
      .fetchIdentity(request);

    return GetIdentityResponse.deserializeBinary(identityResponseBuffer);
  }

  return getIdentityHandler;
}

module.exports = getIdentityHandlerFactory;
