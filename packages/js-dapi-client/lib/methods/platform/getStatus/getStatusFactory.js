const {
  v0: {
    PlatformPromiseClient,
    GetStatusRequest,
  },
} = require('@dashevo/dapi-grpc');

const InvalidResponseError = require('../response/errors/InvalidResponseError');
const GetStatusResponse = require('./GetStatusResponse');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {getStatus}
 */
function getStatusFactory(grpcTransport) {
  /**
   * Fetch node status
   * @typedef {getStatus}
   * @param {DAPIClientOptions & {prove: boolean}} [options]
   * @returns {Promise<GetStatusResponse>}
   */
  async function getStatus(options = {}) {
    const { GetStatusRequestV0 } = GetStatusRequest;
    const getStatusRequest = new GetStatusRequest();

    getStatusRequest.setV0(
      new GetStatusRequestV0(),
    );

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const getStatusResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'getStatus',
          getStatusRequest,
          options,
        );

        if (getStatusResponse.getV0() === undefined) {
          // noinspection ExceptionCaughtLocallyJS
          throw new InvalidResponseError('GetStatusResponseV0 is not defined');
        }

        return GetStatusResponse.createFromProto(getStatusResponse);
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

  return getStatus;
}

module.exports = getStatusFactory;
