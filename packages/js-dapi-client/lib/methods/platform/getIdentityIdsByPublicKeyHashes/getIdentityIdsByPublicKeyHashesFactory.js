const {
  v0: {
    PlatformPromiseClient,
    GetIdentityIdsByPublicKeyHashesRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityIdsByPublicKeyHashesResponse = require('./GetIdentityIdsByPublicKeyHashesResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

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

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentityIdsByPublicKeyHashesResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentityIdsByPublicKeyHashes',
          getIdentityIdsByPublicKeyHashesRequest,
          options,
        );

        return GetIdentityIdsByPublicKeyHashesResponse
          .createFromProto(getIdentityIdsByPublicKeyHashesResponse);
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

  return getIdentityIdsByPublicKeyHashes;
}

module.exports = getIdentityIdsByPublicKeyHashesFactory;
