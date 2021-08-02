const {
  v0: {
    BroadcastStateTransitionRequest,
    PlatformPromiseClient,
  },
} = require('@dashevo/dapi-grpc');
const BroadcastStateTransitionResponse = require('./BroadcastStateTransitionResponse');
const InvalidResponseError = require('../response/errors/InvalidResponseError');

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

    let lastError;

    // TODO: simple retry before the dapi versioning is properly implemented
    for (let i = 0; i < 3; i += 1) {
      try {
        // eslint-disable-next-line no-await-in-loop
        const broadcastStateTransitionResponse = await grpcTransport.request(
          PlatformPromiseClient,
          'broadcastStateTransition',
          broadcastStateTransitionRequest,
          options,
        );
        return BroadcastStateTransitionResponse.createFromProto(broadcastStateTransitionResponse);
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

  return broadcastStateTransition;
}

module.exports = broadcastStateTransitionFactory;
