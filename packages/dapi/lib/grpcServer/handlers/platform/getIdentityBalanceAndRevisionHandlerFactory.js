const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityBalanceAndRevisionResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 * @param {DriveClient} driveClient
 *
 * @return {getIdentityBalanceAndRevisionHandler}
 */
function getIdentityBalanceAndRevisionHandlerFactory(driveClient) {
  /**
   * @typedef getIdentityBalanceAndRevisionHandler
   *
   * @param {Object} call
   *
   * @return {Promise<GetIdentityBalanceAndRevisionResponse>}
   */
  async function getIdentityBalanceAndRevisionHandler(call) {
    const { request } = call;

    if (!request.getId()) {
      throw new InvalidArgumentGrpcError('identity id is not specified');
    }

    const identityResponseBuffer = await driveClient
      .fetchIdentityBalanceAndRevision(request);

    return GetIdentityBalanceAndRevisionResponse.deserializeBinary(identityResponseBuffer);
  }

  return getIdentityBalanceAndRevisionHandler;
}

module.exports = getIdentityBalanceAndRevisionHandlerFactory;
