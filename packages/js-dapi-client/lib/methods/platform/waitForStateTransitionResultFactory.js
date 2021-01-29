const {
  v0: {
    PlatformPromiseClient,
    WaitForStateTransitionResultRequest,
  },
} = require('@dashevo/dapi-grpc');

const cbor = require('cbor');

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

    const error = waitForStateTransitionResultResponse.getError();
    const proof = waitForStateTransitionResultResponse.getProof();

    const result = {};

    if (proof) {
      result.proof = {
        rootTreeProof: Buffer.from(proof.getRootTreeProof()),
        storeTreeProof: Buffer.from(proof.getStoreTreeProof()),
      };
    }

    if (error) {
      let data;
      const rawData = error.getData();
      if (rawData) {
        data = cbor.decode(Buffer.from(rawData));
      }

      result.error = {
        code: error.getCode(),
        message: error.getMessage(),
        data,
      };
    }

    return result;
  }

  return waitForStateTransitionResult;
}

module.exports = waitForStateTransitionResultFactory;
