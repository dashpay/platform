const {
  v0: {
    PlatformPromiseClient,
    GetProtocolVersionUpgradeStateRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetProtocolVersionUpgradeStateResponse = require('./GetProtocolVersionUpgradeStateResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getProtocolVersionUpgradeState}
 */
function getProtocolVersionUpgradeStateFactory(grpcTransport) {
  /**
   * Fetch the version upgrade state
   * @typedef {getProtocolVersionUpgradeState}
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetProtocolVersionUpgradeStateResponse>}
   */
  async function getProtocolVersionUpgradeState(options = {}) {
    const { GetProtocolVersionUpgradeStateRequestV0 } = GetProtocolVersionUpgradeStateRequest;
    const getProtocolVersionUpgradeStateRequest = new GetProtocolVersionUpgradeStateRequest();
    getProtocolVersionUpgradeStateRequest.setV0(
      new GetProtocolVersionUpgradeStateRequestV0()
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getProtocolVersionUpgradeStateResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getProtocolVersionUpgradeState',
          getProtocolVersionUpgradeStateRequest,
          options,
        );

        return GetProtocolVersionUpgradeStateResponse
          .createFromProto(getProtocolVersionUpgradeStateResponse);
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

  return getProtocolVersionUpgradeState;
}

module.exports = getProtocolVersionUpgradeStateFactory;
