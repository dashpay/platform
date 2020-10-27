const {
  v0: {
    PlatformPromiseClient,
    GetIdentityIdsByPublicKeyHashesRequest,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityIdsByPublicKeyHashes}
 */
function getIdentityIdsByPublicKeyHashesFactory(grpcTransport) {
  /**
   * Fetch the identities by public key hashes
   *
   * @typedef {getIdentityIdsByPublicKeyHashes}
   * @param {Buffer[]} publicKeyHashes
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<!Array<Buffer|null>>}
   */
  async function getIdentityIdsByPublicKeyHashes(publicKeyHashes, options = {}) {
    const getIdentityIdsByPublicKeyHashesRequest = new GetIdentityIdsByPublicKeyHashesRequest();
    getIdentityIdsByPublicKeyHashesRequest.setPublicKeyHashesList(
      publicKeyHashes,
    );

    let getIdentityIdsByPublicKeyHashesResponse;
    try {
      getIdentityIdsByPublicKeyHashesResponse = await grpcTransport.request(
        PlatformPromiseClient,
        'getIdentityIdsByPublicKeyHashes',
        getIdentityIdsByPublicKeyHashesRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    return getIdentityIdsByPublicKeyHashesResponse.getIdentityIdsList()
      .map((identityId) => (identityId.length > 0 ? Buffer.from(identityId) : null));
  }

  return getIdentityIdsByPublicKeyHashes;
}

module.exports = getIdentityIdsByPublicKeyHashesFactory;
