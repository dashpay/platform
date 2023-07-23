const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentityByPublicKeyHashesResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {DriveClient} driveClient
 * @return {getIdentityByPublicKeyHashesHandler}
 */
function getIdentityByPublicKeyHashesHandlerFactory(
  driveClient,
) {
  /**
   * @typedef getIdentityByPublicKeyHashesHandler
   * @param {Object} call
   * @return {Promise<GetIdentityByPublicKeyHashesResponse>}
   */
  async function getIdentityByPublicKeyHashesHandler(call) {
    const { request } = call;

    if (request.getPublicKeyHashesList().length === 0) {
      throw new InvalidArgumentGrpcError('No public key hashes were provided');
    }

    const identitiesResponseBuffer = await driveClient
      .fetchIdentityByPublicKeyHashes(request);

    return GetIdentityByPublicKeyHashesResponse.deserializeBinary(identitiesResponseBuffer);
  }

  return getIdentityByPublicKeyHashesHandler;
}

module.exports = getIdentityByPublicKeyHashesHandlerFactory;
