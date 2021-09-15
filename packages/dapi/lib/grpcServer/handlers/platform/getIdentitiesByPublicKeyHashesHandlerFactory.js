const {
  server: {
    error: {
      InvalidArgumentGrpcError,
    },
  },
} = require('@dashevo/grpc-common');

const {
  v0: {
    GetIdentitiesByPublicKeyHashesResponse,
  },
} = require('@dashevo/dapi-grpc');

/**
 *
 * @param {DriveClient} driveClient
 * @return {getIdentitiesByPublicKeyHashesHandler}
 */
function getIdentitiesByPublicKeyHashesHandlerFactory(
  driveClient,
) {
  /**
   * @typedef getIdentitiesByPublicKeyHashesHandler
   * @param {Object} call
   * @return {Promise<GetIdentitiesByPublicKeyHashesResponse>}
   */
  async function getIdentitiesByPublicKeyHashesHandler(call) {
    const { request } = call;

    const publicKeyHashes = request.getPublicKeyHashesList();

    if (publicKeyHashes.length === 0) {
      throw new InvalidArgumentGrpcError('No public key hashes were provided');
    }

    const prove = request.getProve();

    const identitiesResponseBuffer = await driveClient
      .fetchIdentitiesByPublicKeyHashes(publicKeyHashes, prove);

    return GetIdentitiesByPublicKeyHashesResponse.deserializeBinary(identitiesResponseBuffer);
  }

  return getIdentitiesByPublicKeyHashesHandler;
}

module.exports = getIdentitiesByPublicKeyHashesHandlerFactory;
