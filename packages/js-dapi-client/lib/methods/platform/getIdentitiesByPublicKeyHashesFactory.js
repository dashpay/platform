const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesByPublicKeyHashesRequest,
  },
} = require('@dashevo/dapi-grpc');

const grpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentitiesByPublicKeyHashes}
 */
function getIdentitiesByPublicKeyHashesFactory(grpcTransport) {
  /**
   * Fetch the identities by public key hashes
   *
   * @typedef {getIdentitiesByPublicKeyHashes}
   * @param {Buffer[]} publicKeyHashes
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<!Array<Buffer|null>>}
   */
  async function getIdentitiesByPublicKeyHashes(publicKeyHashes, options = {}) {
    const getIdentitiesByPublicKeyHashesRequest = new GetIdentitiesByPublicKeyHashesRequest();
    getIdentitiesByPublicKeyHashesRequest.setPublicKeyHashesList(
      publicKeyHashes,
    );

    let getIdentitiesByPublicKeyHashesResponse;
    try {
      getIdentitiesByPublicKeyHashesResponse = await grpcTransport.request(
        PlatformPromiseClient,
        'getIdentitiesByPublicKeyHashes',
        getIdentitiesByPublicKeyHashesRequest,
        options,
      );
    } catch (e) {
      if (e.code === grpcErrorCodes.NOT_FOUND) {
        return null;
      }

      throw e;
    }

    return getIdentitiesByPublicKeyHashesResponse.getIdentitiesList()
      .map((identity) => (identity.length > 0 ? Buffer.from(identity) : null));
  }

  return getIdentitiesByPublicKeyHashes;
}

module.exports = getIdentitiesByPublicKeyHashesFactory;
