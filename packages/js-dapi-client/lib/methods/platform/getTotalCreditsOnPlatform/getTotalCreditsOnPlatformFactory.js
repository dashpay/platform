const {
  v0: {
    PlatformPromiseClient,
    GetTotalCreditsOnPlatformRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetTotalCreditsOnPlatformResponse = require('./GetTotalCreditsOnPlatformResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getTotalCreditsOnPlatform}
 */
function getTotalCreditsOnPlatformFactory(grpcTransport) {
  /**
   * Fetch the version upgrade votes status
   * @typedef {getTotalCreditsOnPlatform}
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetTotalCreditsOnPlatformResponse>}
   */
  async function getTotalCreditsOnPlatform(options = {}) {
    const {
      GetTotalCreditsOnPlatformRequestV0,
    } = GetTotalCreditsOnPlatformRequest;

    // eslint-disable-next-line max-len
    const getTotalCreditsOnPlatformRequest = new GetTotalCreditsOnPlatformRequest();

    getTotalCreditsOnPlatformRequest.setV0(
      new GetTotalCreditsOnPlatformRequestV0()
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getTotalCreditsOnPlatformResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getTotalCreditsOnPlatform',
          getTotalCreditsOnPlatformRequest,
          options,
        );

        return GetTotalCreditsOnPlatformResponse
          .createFromProto(getTotalCreditsOnPlatformResponse);
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

  return getTotalCreditsOnPlatform;
}

module.exports = getTotalCreditsOnPlatformFactory;
