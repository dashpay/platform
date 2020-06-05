const {
  ApplyStateTransitionRequest,
  PlatformPromiseClient,
} = require('@dashevo/dapi-grpc');

/**
 * @param {GrpcTransport} grpcTransport
 * @returns {broadcastStateTransition}
 */
function broadcastStateTransitionFactory(grpcTransport) {
  /**
   * Broadcast State Transaction
   *
   * @typedef {broadcastStateTransition}
   * @param {Buffer} stateTransition
   * @param {DAPIClientOptions} [options]
   * @returns {Promise<!ApplyStateTransitionResponse>}
   */
  async function broadcastStateTransition(stateTransition, options = {}) {
    const applyStateTransitionRequest = new ApplyStateTransitionRequest();
    applyStateTransitionRequest.setStateTransition(stateTransition);

    return grpcTransport.request(
      PlatformPromiseClient,
      'applyStateTransition',
      applyStateTransitionRequest,
      options,
    );
  }

  return broadcastStateTransition;
}

module.exports = broadcastStateTransitionFactory;
