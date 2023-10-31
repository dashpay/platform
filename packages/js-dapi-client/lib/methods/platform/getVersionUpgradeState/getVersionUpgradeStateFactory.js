const {
  v0: {
    PlatformPromiseClient,
    GetVersionUpgradeStateRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetVersionUpgradeStateResponse = require('./GetVersionUpgradeStateResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getVersionUpgradeState}
 */
function getVersionUpgradeStateFactory(grpcTransport) {
  /**
   * Fetch the version upgrade state
   *
   * @typedef {getVersionUpgradeState}
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetVersionUpgradeStateResponse>}
   */
  async function getVersionUpgradeState(options = {}) {
    const { GetVersionUpgradeStateRequestV0 } = GetVersionUpgradeStateRequest;
    const getVersionUpgradeStateRequest = new GetVersionUpgradeStateRequest();
    getVersionUpgradeStateRequest.setV0(
      new GetVersionUpgradeStateRequestV0()
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getVersionUpgradeStateResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getVersionUpgradeState',
          getVersionUpgradeStateRequest,
          options,
        );

        return GetVersionUpgradeStateResponse
          .createFromProto(getVersionUpgradeStateResponse);
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

  return getVersionUpgradeState;
}

module.exports = getVersionUpgradeStateFactory;
