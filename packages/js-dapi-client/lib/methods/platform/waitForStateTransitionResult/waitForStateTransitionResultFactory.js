const {
  v0: {
    PlatformPromiseClient,
    WaitForStateTransitionResultRequest,
  },
} = require('@dashevo/dapi-grpc');

const WaitForStateTransitionResultResponse = require('./WaitForStateTransitionResultResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

/**
 *
 * @param {GrpcTransport} grpcTransport
 * @returns {waitForStateTransitionResult}
 */
function waitForStateTransitionResultFactory(grpcTransport) {
  /**
   * @typedef waitForStateTransitionResult
   * @param {Buffer} stateTransitionHash
   * @param {DAPIClientOptions & getDocumentsOptions & {prove: boolean}} [options]
   * @returns {Promise<object>}
   */
  async function waitForStateTransitionResult(stateTransitionHash, options = {}) {
    // eslint-disable-next-line no-param-reassign
    options = {
      // Set default timeout
      timeout: 60000,
      prove: false,
      retry: 0,
      throwDeadlineExceeded: true,
      ...options,
    };

    const waitForStateTransitionResultRequest = new WaitForStateTransitionResultRequest();

    waitForStateTransitionResultRequest.setStateTransitionHash(stateTransitionHash);
    waitForStateTransitionResultRequest.setProve(options.prove);

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const waitForStateTransitionResultResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'waitForStateTransitionResult',
          waitForStateTransitionResultRequest,
          options,
        );

        return WaitForStateTransitionResultResponse.createFromProto(
          waitForStateTransitionResultResponse,
        );
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

  return waitForStateTransitionResult;
}

module.exports = waitForStateTransitionResultFactory;
