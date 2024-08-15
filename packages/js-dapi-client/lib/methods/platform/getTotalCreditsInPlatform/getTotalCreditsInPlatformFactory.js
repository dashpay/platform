const {
  v0: {
    PlatformPromiseClient,
    GetTotalCreditsInPlatformRequest,
  },
} = require('@dashevo/dapi-grpc');

const GetTotalCreditsInPlatformResponse = require('./GetTotalCreditsInPlatformResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getTotalCreditsInPlatform}
 */
function getTotalCreditsInPlatformFactory(grpcTransport) {
  /**
   * Fetch the version upgrade votes status
   * @typedef {getTotalCreditsInPlatform}
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetTotalCreditsInPlatformResponse>}
   */
  async function getTotalCreditsInPlatform(options = {}) {
    const {
      GetTotalCreditsInPlatformRequestV0,
    } = GetTotalCreditsInPlatformRequest;

    // eslint-disable-next-line max-len
    const getTotalCreditsInPlatformRequest = new GetTotalCreditsInPlatformRequest();

    getTotalCreditsInPlatformRequest.setV0(
      new GetTotalCreditsInPlatformRequestV0()
        .setProve(!!options.prove),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getTotalCreditsInPlatformResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getTotalCreditsInPlatform',
          getTotalCreditsInPlatformRequest,
          options,
        );

        return GetTotalCreditsInPlatformResponse
          .createFromProto(getTotalCreditsInPlatformResponse);
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

  return getTotalCreditsInPlatform;
}

module.exports = getTotalCreditsInPlatformFactory;
