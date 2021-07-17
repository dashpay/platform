const {
  v0: {
    PlatformPromiseClient,
    GetIdentityIdsByPublicKeyHashesRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityIdsByPublicKeyHashesResponse = require('./GetIdentityIdsByPublicKeyHashesResponse');

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
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentityIdsByPublicKeyHashesResponse>}
   */
  async function getIdentityIdsByPublicKeyHashes(publicKeyHashes, options = {}) {
    const getIdentityIdsByPublicKeyHashesRequest = new GetIdentityIdsByPublicKeyHashesRequest();
    getIdentityIdsByPublicKeyHashesRequest.setPublicKeyHashesList(
      publicKeyHashes,
    );
    getIdentityIdsByPublicKeyHashesRequest.setProve(!!options.prove);

    const getIdentityIdsByPublicKeyHashesResponse = await grpcTransport.request(
      PlatformPromiseClient,
      'getIdentityIdsByPublicKeyHashes',
      getIdentityIdsByPublicKeyHashesRequest,
      options,
    );

    return GetIdentityIdsByPublicKeyHashesResponse
      .createFromProto(getIdentityIdsByPublicKeyHashesResponse);
  }

  return getIdentityIdsByPublicKeyHashes;
}

module.exports = getIdentityIdsByPublicKeyHashesFactory;
