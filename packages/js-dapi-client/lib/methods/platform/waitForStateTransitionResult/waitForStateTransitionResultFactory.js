const {
  v0: {
    PlatformPromiseClient,
    WaitForStateTransitionResultRequest,
  },
} = require('@dashevo/dapi-grpc');

const WaitForStateTransitionResultResponse = require('./WaitForStateTransitionResultResponse');

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
   * @returns {Promise<Object>}
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

    const waitForStateTransitionResultResponse = await grpcTransport.request(
      PlatformPromiseClient,
      'waitForStateTransitionResult',
      waitForStateTransitionResultRequest,
      options,
    );

    return WaitForStateTransitionResultResponse.createFromProto(
      waitForStateTransitionResultResponse,
    );
  }

  return waitForStateTransitionResult;
}

module.exports = waitForStateTransitionResultFactory;
