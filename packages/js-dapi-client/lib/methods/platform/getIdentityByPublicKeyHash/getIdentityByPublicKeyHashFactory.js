const {
  v0: {
    PlatformPromiseClient,
    GetIdentityByPublicKeyHashRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityByPublicKeyHashResponse = require('./GetIdentityByPublicKeyHashResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityByPublicKeyHash}
 */
function getIdentityByPublicKeyHashFactory(grpcTransport) {
  /**
   * Fetch the identity by public key hash
   * @typedef {getIdentityByPublicKeyHash}
   * @param {Buffer} publicKeyHash
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentityByPublicKeyHashResponse>}
   */
  async function getIdentityByPublicKeyHash(publicKeyHash, options = {}) {
    const { GetIdentityByPublicKeyHashRequestV0 } = GetIdentityByPublicKeyHashRequest;
    const getIdentityByPublicKeyHashRequest = new GetIdentityByPublicKeyHashRequest();
    getIdentityByPublicKeyHashRequest.setV0(
      new GetIdentityByPublicKeyHashRequestV0()
        .setPublicKeyHash(publicKeyHash)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentityByPublicKeyHashResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentityByPublicKeyHash',
          getIdentityByPublicKeyHashRequest,
          options,
        );

        return GetIdentityByPublicKeyHashResponse
          .createFromProto(getIdentityByPublicKeyHashResponse);
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

  return getIdentityByPublicKeyHash;
}

module.exports = getIdentityByPublicKeyHashFactory;
