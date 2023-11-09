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
 * @return {getIdentityByPublicKeyHashesHandler}
 */
function getIdentityByPublicKeyHashHandlerFactory(
  driveClient,
) {
  /**
   * @typedef getIdentityByPublicKeyHashesHandler
   * @param {Object} call
   * @return {Promise<GetIdentityByPublicKeyHashResponse>}
   */
  async function getIdentityByPublicKeyHashesHandler(call) {
    const { request } = call;

    if (request.getPublicKeyHash().length === 0) {
      throw new InvalidArgumentGrpcError('No public key hash is provided');
    }

    const identitiesResponseBuffer = await driveClient
      .fetchIdentityByPublicKeyHash(request);

    return GetIdentityByPublicKeyHashResponse.deserializeBinary(identitiesResponseBuffer);
  }

  return getIdentityByPublicKeyHashesHandler;
}

module.exports = getIdentityByPublicKeyHashHandlerFactory;
