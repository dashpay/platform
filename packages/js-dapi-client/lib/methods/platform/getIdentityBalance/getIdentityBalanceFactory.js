const {
  v0: {
    PlatformPromiseClient,
    GetIdentityBalanceRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetIdentityBalanceResponse = require('./GetIdentityBalanceResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getIdentityBalance}
 */
function getIdentityBalanceFactory(grpcTransport) {
  /**
   * Fetch the identity balance by id
   * @typedef {getIdentityBalance}
   * @param {Uint8Array} id
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetIdentityBalanceResponse>}
   */
  async function getIdentityBalance(id, options = {}) {
    const { GetIdentityBalanceRequestV0 } = GetIdentityBalanceRequest;
    const getIdentityBalanceRequest = new GetIdentityBalanceRequest();
    // need to convert objects inherited from Uint8Array to pure Uint8Array as google protobuf
    // doesn't support extended buffers
    // https://github.com/protocolbuffers/protobuf/blob/master/js/binary/utils.js#L1049
    if (id instanceof Uint8Array) {
      // eslint-disable-next-line no-param-reassign
      id = new Uint8Array(id);
    }

    getIdentityBalanceRequest.setV0(
      new GetIdentityBalanceRequestV0()
        .setId(id)
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getIdentityBalanceResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getIdentityBalance',
          getIdentityBalanceRequest,
          options,
        );

        return GetIdentityBalanceResponse.createFromProto(getIdentityBalanceResponse);
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

  return getIdentityBalance;
}

module.exports = getIdentityBalanceFactory;
