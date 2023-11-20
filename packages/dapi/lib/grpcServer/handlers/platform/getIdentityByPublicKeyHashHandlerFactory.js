const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityByPublicKeyHashResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {DriveClient} driveClient
 * @return {getIdentityByPublicKeyHashHandler}
 */
function getIdentityByPublicKeyHashHandlerFactory(
  driveClient,
) {
  /**
   * @typedef getIdentityByPublicKeyHashHandler
   * @param {Object} call
   * @return {Promise<GetIdentityByPublicKeyHashResponse>}
   */
  async function getIdentityByPublicKeyHashHandler(call) {
    const { request } = call;

    if (request.getV0().getPublicKeyHash().length === 0) {
      throw new InvalidArgumentGrpcError('No public key hash is provided');
    }

    const identitiesResponseBuffer = await driveClient
      .fetchIdentityByPublicKeyHash(request);

    return GetIdentityByPublicKeyHashResponse.deserializeBinary(identitiesResponseBuffer);
  }

  return getIdentityByPublicKeyHashHandler;
}

module.exports = getIdentityByPublicKeyHashHandlerFactory;
