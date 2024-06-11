const {
  v0: {
    PlatformPromiseClient,
    GetIdentityContractNonceRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityContractNonceResponse = require('./GetIdentityContractNonceResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityContractNonce}
 */
function getIdentityContractNonceFactory(grpcTransport) {
  /**
   * Fetch the version upgrade votes status
   * @typedef {getIdentityContractNonce}
   * @param {Buffer} identityId
   * @param {Buffer} contractId
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentityContractNonceResponse>}
   */
  async function getIdentityContractNonce(identityId, contractId, options = {}) {
    const {
      GetIdentityContractNonceRequestV0,
    } = GetIdentityContractNonceRequest;

    // eslint-disable-next-line max-len
    const getIdentityContractNonceRequest = new GetIdentityContractNonceRequest();

    if (Buffer.isBuffer(identityId)) {
      // eslint-disable-next-line no-param-reassign
      identityId = Buffer.from(identityId);
    }

    if (Buffer.isBuffer(contractId)) {
      // eslint-disable-next-line no-param-reassign
      contractId = Buffer.from(contractId);
    }

    getIdentityContractNonceRequest.setV0(
      new GetIdentityContractNonceRequestV0()
        .setIdentityId(identityId)
        .setContractId(contractId)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentityContractNonceResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentityContractNonce',
          getIdentityContractNonceRequest,
          options,
        );

        return GetIdentityContractNonceResponse
          .createFromProto(getIdentityContractNonceResponse);
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

  return getIdentityContractNonce;
}

module.exports = getIdentityContractNonceFactory;
