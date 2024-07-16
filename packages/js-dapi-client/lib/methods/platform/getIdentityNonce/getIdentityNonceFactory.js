const {
  v0: {
    PlatformPromiseClient,
    GetIdentityNonceRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityNonceResponse = require('./GetIdentityNonceResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityNonce}
 */
function getIdentityNonceFactory(grpcTransport) {
  /**
   * Fetch the version upgrade votes status
   * @typedef {getIdentityNonce}
   * @param {Buffer} identityId
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentityNonceResponse>}
   */
  async function getIdentityNonce(identityId, options = {}) {
    const {
      GetIdentityNonceRequestV0,
    } = GetIdentityNonceRequest;

    // eslint-disable-next-line max-len
    const getIdentityNonceRequest = new GetIdentityNonceRequest();

    if (Buffer.isBuffer(identityId)) {
      // eslint-disable-next-line no-param-reassign
      identityId = Buffer.from(identityId);
    }

    getIdentityNonceRequest.setV0(
      new GetIdentityNonceRequestV0()
        .setIdentityId(identityId)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentityNonceResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentityNonce',
          getIdentityNonceRequest,
          options,
        );

        return GetIdentityNonceResponse
          .createFromProto(getIdentityNonceResponse);
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

  return getIdentityNonce;
}

module.exports = getIdentityNonceFactory;
