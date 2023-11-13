const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityKeysResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @return {getIdentityKeysHandler}
 */
function getIdentityKeysHandlerFactory(driveClient) {
  /**
   * @typedef getIdentityKeysHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentityKeysResponse>}
   */
  async function getIdentityKeysHandler(call) {
    const { request } = call;

    if (!request.getV0().getIdentityId()) {
      throw new InvalidArgumentGrpcError('identity id is not specified');
    }

    const identityResponseBuffer = await driveClient
      .fetchIdentityKeys(request);

    return GetIdentityKeysResponse.deserializeBinary(identityResponseBuffer);
  }

  return getIdentityKeysHandler;
}

module.exports = getIdentityKeysHandlerFactory;
