const {
  v0: {
    PlatformPromiseClient,
    GetIdentitiesByPublicKeyHashesRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentitiesByPublicKeyHashesResponse = require('./GetIdentitiesByPublicKeyHashesResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

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
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentitiesByPublicKeyHashesResponse>}
   */
  async function getIdentitiesByPublicKeyHashes(publicKeyHashes, options = {}) {
    const getIdentitiesByPublicKeyHashesRequest = new GetIdentitiesByPublicKeyHashesRequest();
    getIdentitiesByPublicKeyHashesRequest.setPublicKeyHashesList(
      publicKeyHashes,
    );
    getIdentitiesByPublicKeyHashesRequest.setProve(!!options.prove);

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentitiesByPublicKeyHashesResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentitiesByPublicKeyHashes',
          getIdentitiesByPublicKeyHashesRequest,
          options,
        );

        return GetIdentitiesByPublicKeyHashesResponse
          .createFromProto(getIdentitiesByPublicKeyHashesResponse);
      } catch (e) {
        if (e instanceof InvalidResponseError) {
          lastError = e;
        } else {
          throw e;
        }
      }
    }

    // If we made it past the cycle it means that the retry didn't work,
    // and we're throwing the last error encountered
    throw lastError;
  }

  return getIdentitiesByPublicKeyHashes;
}

module.exports = getIdentitiesByPublicKeyHashesFactory;
