const {
  v0: {
    PlatformPromiseClient,
    GetConsensusParamsRequest,
  },
} = require('@dashevo/dapi-grpc');
const InvalidResponseError = require('../response/errors/InvalidResponseError');
const GetConsensusParamsResponse = require('./getConsensusParamsResponse');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getConsensusParams}
 */
function getConsensusParamsFactory(grpcTransport) {
  /**
   * Fetch Consensus params
   *
   * @typedef getConsensusParams
   * @param {number} [height]
   * @param {prove: boolean} [options]
   * @returns {Promise<GetConsensusParamsResponse>}
   */
  async function getConsensusParams(height = undefined, options = {}) {
    const getConsensusParamsRequest = new GetConsensusParamsRequest();
    if (height !== undefined) {
      getConsensusParamsRequest.setHeight(height);
    }

    getConsensusParamsRequest.setProve(!!options.prove);

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getConsensusParamsResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getConsensusParams',
          getConsensusParamsRequest,
          options,
        );

        return GetConsensusParamsResponse.createFromProto(getConsensusParamsResponse);
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

  return getConsensusParams;
}

module.exports = getConsensusParamsFactory;
