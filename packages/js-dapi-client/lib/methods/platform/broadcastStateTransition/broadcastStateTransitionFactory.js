const {
  v0: {
    BroadcastStateTransitionRequest,
    PlatformPromiseClient,
  },
} = require('@dashevo/dapi-grpc');
const BroadcastStateTransitionResponse = require('./BroadcastStateTransitionResponse');

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
   * @returns {Promise<!BroadcastStateTransitionResponse>}
   */
  async function broadcastStateTransition(stateTransition, options = {}) {
    const broadcastStateTransitionRequest = new BroadcastStateTransitionRequest();
    broadcastStateTransitionRequest.setStateTransition(stateTransition);

    const broadcastStateTransitionResponse = await grpcTransport.request(
      PlatformPromiseClient,
      'broadcastStateTransition',
      broadcastStateTransitionRequest,
      options,
    );

    return BroadcastStateTransitionResponse.createFromProto(broadcastStateTransitionResponse);
  }

  return broadcastStateTransition;
}

module.exports = broadcastStateTransitionFactory;
