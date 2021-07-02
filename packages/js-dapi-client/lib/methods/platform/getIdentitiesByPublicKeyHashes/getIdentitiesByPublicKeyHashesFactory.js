const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesByPublicKeyHashesRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentitiesByPublicKeyHashesResponse = require('./GetIdentitiesByPublicKeyHashesResponse');

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
   * @returns {Promise<GetIdentitiesByPublicKeyHashesResponse>}
   */
  async function getIdentitiesByPublicKeyHashes(publicKeyHashes, options = {}) {
    const getIdentitiesByPublicKeyHashesRequest = new GetIdentitiesByPublicKeyHashesRequest();
    getIdentitiesByPublicKeyHashesRequest.setPublicKeyHashesList(
      publicKeyHashes,
    );

    const getIdentitiesByPublicKeyHashesResponse = await grpcTransport.request(
      PlatformPromiseClient,
      'getIdentitiesByPublicKeyHashes',
      getIdentitiesByPublicKeyHashesRequest,
      options,
    );

    return GetIdentitiesByPublicKeyHashesResponse
      .createFromProto(getIdentitiesByPublicKeyHashesResponse);
  }

  return getIdentitiesByPublicKeyHashes;
}

module.exports = getIdentitiesByPublicKeyHashesFactory;
